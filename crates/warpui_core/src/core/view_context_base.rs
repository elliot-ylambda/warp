//! The backend-generic base shared by the GUI [`ViewContext`](crate::ViewContext)
//! and TUI [`TuiViewContext`](crate::core::TuiViewContext) facades.
//!
//! `BaseViewContext` owns the backend-neutral view-context operations (model
//! access, event emission, notification, focus, subscriptions/observations, and
//! async task registration) exactly once, so the two facades reduce to thin
//! wrappers that supply only the backend-specific pieces: the concrete-view
//! downcast and concrete-context construction inside callbacks.

use std::any::Any;

use futures::future::{AbortHandle, Abortable};
use futures::{Future, FutureExt};

use super::{
    ActiveBackend, AppContextImpl, Backend, Observation, ObservationFromViewCallback, Subscription,
    SubscriptionFromViewCallback, TaskCallback,
};
use crate::r#async::{SpawnableOutput, SpawnedFutureHandle, SpawnedLocalStream};
use crate::{Action, AppContext, Effect, Entity, EntityId, ModelContext, ModelHandle, WindowId};

/// The active backend's type-erased per-window view object: `dyn AnyView` for
/// the GUI backend, `dyn AnyTuiView` for the TUI backend. The shared async/
/// dispatch helpers below hand this to facade-supplied callbacks, which recover
/// the concrete view via [`AnyView::as_any_mut`](crate::AnyView::as_any_mut).
type ActiveAnyView = <ActiveBackend as Backend>::AnyView;

/// Combines a view's identity (`window_id` + `view_id`) with a mutable borrow of
/// the shared application context, and owns the backend-neutral operations both
/// view-context facades need.
///
/// The struct is generic over `B: Backend` (it holds `&mut AppContextImpl<B>`),
/// but its methods are implemented for the active backend, since they call
/// `AppContext`-level machinery that is not (yet) backend-generic.
pub(crate) struct BaseViewContext<'a, B: Backend> {
    app: &'a mut AppContextImpl<B>,
    window_id: WindowId,
    view_id: EntityId,
}

impl<'a> BaseViewContext<'a, ActiveBackend> {
    pub(crate) fn new(app: &'a mut AppContext, window_id: WindowId, view_id: EntityId) -> Self {
        Self {
            app,
            window_id,
            view_id,
        }
    }

    pub(crate) fn focus_self(&mut self) {
        let (window_id, view_id) = (self.window_id, self.view_id);
        self.focus_view(window_id, view_id);
    }

    pub(crate) fn focus_view(&mut self, window_id: WindowId, view_id: EntityId) {
        self.app
            .pending_effects
            .push_back(Effect::Focus { window_id, view_id });
    }

    pub(crate) fn add_model<S, F>(&mut self, build_model: F) -> ModelHandle<S>
    where
        S: Entity,
        F: FnOnce(&mut ModelContext<S>) -> S,
    {
        self.app.add_model(build_model)
    }

    pub(crate) fn emit_event(&mut self, payload: Box<dyn Any>) {
        self.app.pending_effects.push_back(Effect::Event {
            entity_id: self.view_id,
            payload,
        });
    }

    pub(crate) fn notify(&mut self) {
        self.app
            .pending_effects
            .push_back(Effect::ViewNotification {
                window_id: self.window_id,
                view_id: self.view_id,
            });
    }

    pub(crate) fn dispatch_typed_action_deferred(&mut self, action: Box<dyn Action>) {
        self.app.pending_effects.push_back(Effect::TypedAction {
            window_id: self.window_id,
            view_id: self.view_id,
            action,
        });
    }

    pub(crate) fn add_view_subscription(
        &mut self,
        emitter_id: EntityId,
        callback: Box<SubscriptionFromViewCallback>,
    ) {
        self.app
            .subscriptions
            .entry(emitter_id)
            .or_default()
            .push(Subscription::FromView {
                window_id: self.window_id,
                view_id: self.view_id,
                callback,
            });
    }

    pub(crate) fn add_view_observation(
        &mut self,
        observed_id: EntityId,
        callback: Box<ObservationFromViewCallback>,
    ) {
        self.app
            .observations
            .entry(observed_id)
            .or_default()
            .push(Observation::FromView {
                window_id: self.window_id,
                view_id: self.view_id,
                callback,
            });
    }

    /// Schedules `future` on the main thread and registers `run` to be invoked
    /// (with the owning view, type-erased) once it resolves. Returns a future
    /// that completes after `run` runs. The concrete-view downcast and concrete
    /// context are supplied by `run`, keeping this body backend-neutral.
    fn spawn_view_future<S, R>(&mut self, future: S, run: R) -> impl Future<Output = ()>
    where
        S: 'static + Future,
        S::Output: 'static,
        R: 'static + FnOnce(&mut ActiveAnyView, S::Output, &mut AppContext, WindowId, EntityId),
    {
        let (tx, rx) = futures::channel::oneshot::channel();
        let window_id = self.window_id;
        let view_id = self.view_id;

        let task_id = self.app.spawn_local(future);
        self.app.task_callbacks.insert(
            task_id,
            TaskCallback::ViewFromFuture {
                window_id,
                view_id,
                callback: Box::new(move |view, output, app, window_id, view_id| {
                    let output = *output
                        .downcast()
                        .expect("statically enforced by spawn generics");
                    run(view, output, app, window_id, view_id);
                    let _ = tx.send(());
                }),
            },
        );

        async move {
            if rx.await.is_err() {
                log::error!("sender unexpectedly dropped before receiver");
            }
        }
    }

    /// Shared implementation of the facades' `spawn`/`spawn_abortable`: runs
    /// `future` on a background thread and dispatches the (possibly aborted)
    /// result back to the view on the main thread. `on_resolve`/`on_abort`
    /// receive the type-erased view and reconstruct the concrete context.
    pub(crate) fn spawn_abortable<S, OnResolve, OnAbort>(
        &mut self,
        future: S,
        on_resolve: OnResolve,
        on_abort: OnAbort,
    ) -> SpawnedFutureHandle
    where
        S: crate::r#async::Spawnable,
        <S as Future>::Output: SpawnableOutput,
        OnResolve: 'static
            + FnOnce(&mut ActiveAnyView, <S as Future>::Output, &mut AppContext, WindowId, EntityId),
        OnAbort: 'static + FnOnce(&mut ActiveAnyView, &mut AppContext, WindowId, EntityId),
    {
        let (tx, rx) = futures::channel::oneshot::channel();

        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        self.app
            .background_executor()
            .spawn_boxed(Box::pin(async move {
                let abortable = Abortable::new(future, abort_registration);
                if tx.send(abortable.await).is_err() {
                    log::error!("Error sending background task result to main thread");
                }
            }))
            .detach();

        let future = self.spawn_view_future(rx, move |view, rx_result, app, window_id, view_id| {
            let output = match rx_result {
                Ok(output) => output,
                Err(_) => {
                    log::error!("sender unexpectedly dropped before receiver");
                    on_abort(view, app, window_id, view_id);
                    return;
                }
            };

            // `Ok` means the future ran to completion; `Err(Aborted)` means it
            // was aborted before resolving.
            match output {
                Ok(output) => on_resolve(view, output, app, window_id, view_id),
                Err(_) => on_abort(view, app, window_id, view_id),
            }
        });

        let future_id = self.app.register_spawned_future(future.boxed());
        SpawnedFutureHandle::new(abort_handle, future_id)
    }

    /// Shared implementation of the facades' `spawn_stream_local`: polls
    /// `stream` on the main thread, invoking `on_item` per item and `on_done`
    /// on completion, each with the type-erased view.
    pub(crate) fn spawn_view_stream<S, OnItem, OnDone>(
        &mut self,
        stream: S,
        mut on_item: OnItem,
        mut on_done: OnDone,
    ) -> SpawnedLocalStream
    where
        S: 'static + crate::r#async::Stream,
        S::Item: SpawnableOutput,
        OnItem: 'static + FnMut(&mut ActiveAnyView, S::Item, &mut AppContext, WindowId, EntityId),
        OnDone: 'static + FnMut(&mut ActiveAnyView, &mut AppContext, WindowId, EntityId),
    {
        let (tx, rx) = futures::channel::oneshot::channel();
        let window_id = self.window_id;
        let view_id = self.view_id;

        let task_id = self.app.spawn_stream_local(stream, tx);
        self.app.task_callbacks.insert(
            task_id,
            TaskCallback::ViewFromStream {
                window_id,
                view_id,
                on_item: Box::new(move |view, output, app, window_id, view_id| {
                    let output = *output
                        .downcast()
                        .expect("statically enforced by spawn_stream generics");
                    on_item(view, output, app, window_id, view_id);
                }),
                on_done: Box::new(move |view, app, window_id, view_id| {
                    on_done(view, app, window_id, view_id);
                }),
            },
        );

        SpawnedLocalStream::new(
            async move {
                if rx.await.is_err() {
                    log::error!("sender unexpectedly dropped before receiver");
                }
            }
            .boxed_local(),
        )
    }
}
