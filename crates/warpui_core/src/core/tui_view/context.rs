use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use futures::Future;

use super::handle::{TuiReadView, TuiUpdateView, TuiViewAsRef, TuiViewHandle, WeakTuiViewHandle};
use super::{TuiTypedActionView, TuiView};
use crate::core::BaseViewContext;
use crate::r#async::executor::{Background, Foreground};
use crate::r#async::{SpawnableOutput, SpawnedFutureHandle, SpawnedLocalStream};
use crate::{
    Action, AppContext, Entity, EntityId, GetSingletonModelHandle, ModelAsRef, ModelContext,
    ModelHandle, ReadModel, UpdateModel, WindowId,
};

/// The TUI analogue of [`ViewContext`](crate::ViewContext): combines a view's
/// identity with mutable access to the shared application context, exposing the
/// same backend-agnostic operations (model access, async, subscriptions,
/// observation, typed-action dispatch) as the GUI view context.
pub struct TuiViewContext<'a, T: ?Sized> {
    app: &'a mut AppContext,
    window_id: WindowId,
    view_id: EntityId,
    view_type: PhantomData<T>,
}

impl<'a, T: TuiView> TuiViewContext<'a, T> {
    pub(in crate::core) fn new(
        app: &'a mut AppContext,
        window_id: WindowId,
        view_id: EntityId,
    ) -> Self {
        Self {
            app,
            window_id,
            view_id,
            view_type: PhantomData,
        }
    }

    /// Borrows the backend-neutral [`BaseViewContext`] this facade delegates to.
    fn base(&mut self) -> BaseViewContext<'_, crate::core::ActiveBackend> {
        BaseViewContext::new(self.app, self.window_id, self.view_id)
    }

    pub fn handle(&self) -> WeakTuiViewHandle<T> {
        WeakTuiViewHandle::new(self.view_id)
    }

    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    pub fn view_id(&self) -> EntityId {
        self.view_id
    }

    pub fn is_self_focused(&self) -> bool {
        self.app.check_view_focused(self.window_id, &self.view_id)
    }

    pub fn focus<S: TuiView>(&mut self, handle: &TuiViewHandle<S>) {
        let window_id = handle.window_id(self.app);
        let view_id = handle.id();
        self.base().focus_view(window_id, view_id);
    }

    pub fn focus_self(&mut self) {
        self.base().focus_self();
    }

    pub fn add_model<S, F>(&mut self, build_model: F) -> ModelHandle<S>
    where
        S: Entity,
        F: FnOnce(&mut ModelContext<S>) -> S,
    {
        self.base().add_model(build_model)
    }

    pub fn add_view<S, F>(&mut self, build_view: F) -> TuiViewHandle<S>
    where
        S: TuiView,
        F: FnOnce(&mut TuiViewContext<S>) -> S,
    {
        self.app.add_tui_view(self.window_id, build_view)
    }

    pub fn add_typed_action_view<V, F>(&mut self, build_view: F) -> TuiViewHandle<V>
    where
        V: TuiTypedActionView,
        F: FnOnce(&mut TuiViewContext<V>) -> V,
    {
        self.app
            .add_typed_action_tui_view_with_parent(self.window_id, build_view, self.view_id)
    }

    pub fn subscribe_to_model<E, F>(&mut self, handle: &ModelHandle<E>, mut callback: F)
    where
        E: Entity,
        E::Event: 'static,
        F: 'static + FnMut(&mut T, ModelHandle<E>, &E::Event, &mut TuiViewContext<T>),
    {
        let emitter_handle = handle.downgrade();
        self.base().add_view_subscription(
            handle.id(),
            Box::new(move |view, payload, app, window_id, view_id| {
                if let Some(emitter_handle) = emitter_handle.upgrade(app) {
                    let view = view.downcast_mut().expect("downcast is type safe");
                    let payload = payload.downcast_ref().expect("downcast is type safe");
                    let mut ctx = TuiViewContext::new(app, window_id, view_id);
                    callback(view, emitter_handle, payload, &mut ctx);
                }
            }),
        );
    }

    pub fn subscribe_to_view<V, F>(&mut self, handle: &TuiViewHandle<V>, mut callback: F)
    where
        V: TuiView,
        V::Event: 'static,
        F: 'static + FnMut(&mut T, TuiViewHandle<V>, &V::Event, &mut TuiViewContext<T>),
    {
        let emitter_handle = handle.downgrade();
        self.base().add_view_subscription(
            handle.id(),
            Box::new(move |view, payload, app, window_id, view_id| {
                if let Some(emitter_handle) = emitter_handle.upgrade(app) {
                    let view = view.downcast_mut().expect("downcast is type safe");
                    let payload = payload.downcast_ref().expect("downcast is type safe");
                    let mut ctx = TuiViewContext::new(app, window_id, view_id);
                    callback(view, emitter_handle, payload, &mut ctx);
                }
            }),
        );
    }

    pub fn observe<S, F>(&mut self, handle: &ModelHandle<S>, mut callback: F)
    where
        S: Entity,
        F: 'static + FnMut(&mut T, ModelHandle<S>, &mut TuiViewContext<T>),
    {
        self.base().add_view_observation(
            handle.id(),
            Box::new(move |view, observed_id, app, window_id, view_id| {
                let view = view.downcast_mut().expect("downcast is type safe");
                let observed = ModelHandle::new(observed_id, &app.ref_counts);
                let mut ctx = TuiViewContext::new(app, window_id, view_id);
                callback(view, observed, &mut ctx);
            }),
        );
    }

    /// Emits the provided event on this view, to be delivered to any subscribers.
    pub fn emit(&mut self, payload: T::Event) {
        self.base().emit_event(Box::new(payload));
    }

    /// Notifies the framework that this view is dirty and needs re-rendering.
    pub fn notify(&mut self) {
        self.base().notify();
    }

    /// Dispatches a typed action to this view through the shared dispatch path.
    ///
    /// The TUI backend has no layout-derived responder chain (that is a GUI
    /// presenter concept), so the action is dispatched to this view directly.
    pub fn dispatch_typed_action(&mut self, action: &dyn Action) {
        self.app
            .dispatch_typed_action(self.window_id, &[self.view_id], action, log::Level::Info);
    }

    /// Defers dispatching a typed action until effects are flushed.
    pub fn dispatch_typed_action_deferred<A: Action + 'static>(&mut self, action: A) {
        self.base().dispatch_typed_action_deferred(Box::new(action));
    }

    /// Schedules a future on a background thread, invoking `callback` on the
    /// main thread upon completion.
    pub fn spawn<S, F, U>(&mut self, future: S, callback: F) -> SpawnedFutureHandle
    where
        S: crate::r#async::Spawnable,
        <S as Future>::Output: crate::r#async::SpawnableOutput,
        F: 'static + FnOnce(&mut T, <S as Future>::Output, &mut TuiViewContext<T>) -> U,
        U: 'static,
    {
        self.spawn_abortable::<S, _, _>(
            future,
            |view, output, ctx| {
                callback(view, output, ctx);
            },
            |_, _| {},
        )
    }

    /// Schedules a future on a background thread, invoking `on_resolve` on the
    /// main thread upon completion or `on_abort` if it was aborted.
    pub fn spawn_abortable<S, F, A>(
        &mut self,
        future: S,
        on_resolve: F,
        on_abort: A,
    ) -> SpawnedFutureHandle
    where
        S: crate::r#async::Spawnable,
        <S as Future>::Output: crate::r#async::SpawnableOutput,
        F: 'static + FnOnce(&mut T, <S as Future>::Output, &mut TuiViewContext<T>),
        A: 'static + FnOnce(&mut T, &mut TuiViewContext<T>),
    {
        self.base().spawn_abortable(
            future,
            move |view, output, app, window_id, view_id| {
                let view = view
                    .as_any_mut()
                    .downcast_mut()
                    .expect("statically enforced by spawn_abortable generics");
                let mut ctx = TuiViewContext::new(app, window_id, view_id);
                on_resolve(view, output, &mut ctx);
            },
            move |view, app, window_id, view_id| {
                let view = view
                    .as_any_mut()
                    .downcast_mut()
                    .expect("statically enforced by spawn_abortable generics");
                let mut ctx = TuiViewContext::new(app, window_id, view_id);
                on_abort(view, &mut ctx);
            },
        )
    }

    /// Schedules a stream to be polled on the main thread, invoking callbacks on
    /// each item and on completion.
    pub fn spawn_stream_local<S, F, G>(
        &mut self,
        stream: S,
        mut on_item: F,
        mut on_done: G,
    ) -> SpawnedLocalStream
    where
        S: 'static + crate::r#async::Stream,
        S::Item: SpawnableOutput,
        F: 'static + FnMut(&mut T, S::Item, &mut TuiViewContext<T>),
        G: 'static + FnMut(&mut T, &mut TuiViewContext<T>),
    {
        self.base().spawn_view_stream(
            stream,
            move |view, output, app, window_id, view_id| {
                let view = view
                    .as_any_mut()
                    .downcast_mut()
                    .expect("statically enforced by spawn_stream_local generics");
                let mut ctx = TuiViewContext::new(app, window_id, view_id);
                on_item(view, output, &mut ctx);
            },
            move |view, app, window_id, view_id| {
                let view = view
                    .as_any_mut()
                    .downcast_mut()
                    .expect("statically enforced by spawn_stream_local generics");
                let mut ctx = TuiViewContext::new(app, window_id, view_id);
                on_done(view, &mut ctx);
            },
        )
    }

    pub fn foreground_executor(&self) -> &Rc<Foreground> {
        self.app.foreground_executor()
    }

    pub fn background_executor(&self) -> &Arc<Background> {
        self.app.background_executor()
    }
}

impl<T> std::ops::Deref for TuiViewContext<'_, T> {
    type Target = AppContext;

    fn deref(&self) -> &Self::Target {
        self.app
    }
}

impl<T> std::ops::DerefMut for TuiViewContext<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.app
    }
}

impl<V> ModelAsRef for TuiViewContext<'_, V> {
    fn model<T: Entity>(&self, handle: &ModelHandle<T>) -> &T {
        self.app.model(handle)
    }
}

impl<V> ReadModel for TuiViewContext<'_, V> {
    fn read_model<T, F, S>(&self, handle: &ModelHandle<T>, read: F) -> S
    where
        T: Entity,
        F: FnOnce(&T, &AppContext) -> S,
    {
        self.app.read_model(handle, read)
    }
}

impl<V: TuiView> UpdateModel for TuiViewContext<'_, V> {
    fn update_model<T, F, S>(&mut self, handle: &ModelHandle<T>, update: F) -> S
    where
        T: Entity,
        F: FnOnce(&mut T, &mut ModelContext<T>) -> S,
    {
        self.app.update_model(handle, update)
    }
}

impl<V: TuiView> TuiViewAsRef for TuiViewContext<'_, V> {
    fn tui_view<T: TuiView>(&self, handle: &TuiViewHandle<T>) -> &T {
        self.app.tui_view(handle)
    }

    fn try_tui_view<T: TuiView>(&self, handle: &TuiViewHandle<T>) -> Option<&T> {
        self.app.try_tui_view(handle)
    }
}

impl<V: TuiView> TuiReadView for TuiViewContext<'_, V> {
    fn read_tui_view<T, F, S>(&self, handle: &TuiViewHandle<T>, read: F) -> S
    where
        T: TuiView,
        F: FnOnce(&T, &AppContext) -> S,
    {
        self.app.read_tui_view(handle, read)
    }
}

impl<V: TuiView> TuiUpdateView for TuiViewContext<'_, V> {
    fn update_tui_view<T, F, S>(&mut self, handle: &TuiViewHandle<T>, update: F) -> S
    where
        T: TuiView,
        F: FnOnce(&mut T, &mut TuiViewContext<T>) -> S,
    {
        self.app.update_tui_view(handle, update)
    }
}

impl<V: TuiView> GetSingletonModelHandle for TuiViewContext<'_, V> {
    fn get_singleton_model_handle<T: crate::SingletonEntity>(&self) -> ModelHandle<T> {
        self.app.get_singleton_model_handle()
    }
}
