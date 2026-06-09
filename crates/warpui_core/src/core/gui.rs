//! GUI-backend-specific `App`/`AppContext` API.
//!
//! This module holds the `#[cfg(not(feature = "tui"))]` methods and trait impls
//! that were previously scattered through `app.rs` behind per-item cfgs. The
//! single `#[cfg(not(feature = "tui"))] mod gui;` guard in `core/mod.rs` now
//! gates the entire GUI fork; nothing here changes behavior or public API.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use anyhow::{anyhow, Result};
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::Vector2F;

use super::{autotracking, ActionType, AddWindowOptions, App, AppContext, ViewType, Window};
use crate::accessibility::ActionAccessibilityContent;
use crate::platform::{WindowBounds, WindowStyle};
use crate::{
    AccessibilityData, AnyView, BackendView, Element, EntityId,
    NextNewWindowsHasThisWindowsBoundsUponClose, TypedActionView, View, ViewContext, ViewHandle,
    WindowId,
};

impl App {
    /// Adds an action with a given handler that is executed when the action is dispatched. The
    /// action handler should return whether the action was handled. If it returns false this means
    /// a parent view that listens on the same action name will receive the action.
    pub fn add_action<S, V, T, F>(&self, name: S, handler: F)
    where
        S: Into<String>,
        V: View,
        T: Any,
        F: 'static + FnMut(&mut V, &T, &mut ViewContext<V>) -> bool,
    {
        self.0.borrow_mut().add_action(name, handler);
    }

    pub fn add_window_with_bounds<T, F>(
        &mut self,
        style: WindowStyle,
        bounds: WindowBounds,
        build_root_view: F,
    ) -> (WindowId, ViewHandle<T>)
    where
        T: View + TypedActionView,
        F: FnOnce(&mut ViewContext<T>) -> T,
    {
        self.0.borrow_mut().add_window(
            AddWindowOptions {
                window_style: style,
                window_bounds: bounds,
                ..Default::default()
            },
            build_root_view,
        )
    }

    pub fn add_window<T, F>(
        &mut self,
        style: WindowStyle,
        build_root_view: F,
    ) -> (WindowId, ViewHandle<T>)
    where
        T: View + TypedActionView,
        F: FnOnce(&mut ViewContext<T>) -> T,
    {
        self.add_window_with_bounds(style, WindowBounds::Default, build_root_view)
    }

    pub fn add_view<T, F>(&mut self, window_id: WindowId, build_view: F) -> ViewHandle<T>
    where
        T: View,
        F: FnOnce(&mut ViewContext<T>) -> T,
    {
        let mut state = self.0.borrow_mut();
        state.pending_flushes += 1;
        let handle = state.add_view(window_id, build_view);
        state.flush_effects();
        handle
    }

    pub fn add_typed_action_view<V, F>(
        &mut self,
        window_id: WindowId,
        build_view: F,
    ) -> ViewHandle<V>
    where
        V: TypedActionView + View,
        F: FnOnce(&mut ViewContext<V>) -> V,
    {
        self.0
            .borrow_mut()
            .add_typed_action_view(window_id, build_view)
    }

    pub fn add_option_view<T, F>(
        &mut self,
        window_id: WindowId,
        build_view: F,
    ) -> Option<ViewHandle<T>>
    where
        T: View,
        F: FnOnce(&mut ViewContext<T>) -> Option<T>,
    {
        let mut state = self.0.borrow_mut();
        state.pending_flushes += 1;
        let handle = state.add_option_view(window_id, build_view);
        state.flush_effects();
        handle
    }
}

impl AppContext {
    /// Internal helper method to store the handler for a `TypedActionView` being registered
    ///
    /// Creates a handler which will dispatch to `TypedActionView::handle_action` for the given
    /// View + Action combination.
    fn add_typed_action<V>(&mut self)
    where
        V: TypedActionView + View,
    {
        let handler = Box::new(
            |view: &mut (dyn AnyView + 'static),
             action: &dyn Any,
             app: &mut AppContext,
             window_id: WindowId,
             view_id: EntityId| {
                let is_screen_reader_enabled = app
                    .platform_delegate
                    .is_screen_reader_enabled()
                    .unwrap_or(false);
                // Safety: The handler is stored in a map keyed on both the ActionType and the
                // ViewType, so we will only call it if both match, making the downcasts safe
                let action = action
                    .downcast_ref()
                    .expect("Handlers are hashed by action type");
                let view = view
                    .as_any_mut()
                    .downcast_mut()
                    .expect("Handlers are hashed by view type");
                let mut ctx = ViewContext::new(app, window_id, view_id);
                V::handle_action(view, action, &mut ctx);
                if is_screen_reader_enabled {
                    match V::action_accessibility_contents(view, action, &mut ctx) {
                        ActionAccessibilityContent::CustomFn(f) => {
                            app.platform_delegate.set_accessibility_contents(
                                f(action).with_verbosity(app.a11y_verbosity),
                            );
                        }
                        ActionAccessibilityContent::Custom(content) => {
                            app.platform_delegate.set_accessibility_contents(
                                content.with_verbosity(app.a11y_verbosity),
                            );
                        }
                        ActionAccessibilityContent::Empty => {}
                    };
                }
            },
        );

        // Insert the action handler for this view into the `typed_actions` hash
        // We only need to do this once per View type, since the handler is the same for every
        // instance.
        self.typed_actions
            .entry(ActionType::of::<V::Action>())
            .or_default()
            .entry(ViewType::of::<V>())
            .or_insert(handler);
    }

    pub fn add_action<S, V, T, F>(&mut self, name: S, mut handler: F)
    where
        S: Into<String>,
        V: View,
        T: Any,
        F: 'static + FnMut(&mut V, &T, &mut ViewContext<V>) -> bool,
    {
        let name = name.into();
        let name_clone = name.clone();
        let handler = Box::new(
            move |view: &mut (dyn AnyView + 'static),
                  arg: &dyn Any,
                  app: &mut AppContext,
                  window_id: WindowId,
                  view_id: EntityId| {
                match arg.downcast_ref() {
                    Some(arg) => {
                        let mut ctx = ViewContext::new(app, window_id, view_id);
                        handler(
                            view.as_any_mut()
                                .downcast_mut()
                                .expect("downcast is type safe"),
                            arg,
                            &mut ctx,
                        )
                    }
                    None => {
                        log::error!("Could not downcast argument for action {name_clone}");
                        false
                    }
                }
            },
        );

        self.actions
            .entry(TypeId::of::<V>())
            .or_default()
            .entry(name)
            .or_default()
            .push(handler);
    }

    /// Returns the [`AccessibilityData`] of the focused view, or a parent of that view in its
    /// responder chain.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub fn focused_view_accessibility_data(
        &mut self,
        window_id: WindowId,
    ) -> Option<AccessibilityData> {
        let responder_chain = self.get_responder_chain(window_id);
        for view_id in responder_chain {
            let window = self.windows.get_mut(&window_id)?;
            let view = window.views.remove(&view_id)?;
            let accessibility_data = view.accessibility_data(self, window_id, view_id);

            if let Some(window) = self.windows.get_mut(&window_id) {
                window.views.insert(view_id, view);
            }

            if let Some(accessibility_data) = accessibility_data {
                return Some(accessibility_data);
            }
        }
        None
    }

    /// Creates a new window with the view returned by the `build_root_view` function as its root
    /// view.
    pub fn add_window<T, F>(
        &mut self,
        options: AddWindowOptions,
        build_root_view: F,
    ) -> (WindowId, ViewHandle<T>)
    where
        T: View + TypedActionView,
        F: FnOnce(&mut ViewContext<T>) -> T,
    {
        self.insert_window(options, build_root_view)
    }

    fn insert_window<T, F>(
        &mut self,
        add_window_options: AddWindowOptions,
        build_root_view: F,
    ) -> (WindowId, ViewHandle<T>)
    where
        T: View + TypedActionView,
        F: FnOnce(&mut ViewContext<T>) -> T,
    {
        let (window_id, _root_view_id) =
            self.insert_window_internal(None, add_window_options, |window_id, ctx| {
                ctx.windows.insert(window_id, Window::default());
                let root_handle = ctx.add_typed_action_view(window_id, build_root_view);
                let root_view_id = root_handle.id();
                ctx.windows
                    .get_mut(&window_id)
                    .expect("this window was just inserted and should still exist")
                    .root_view = Some(root_handle.into());
                root_view_id
            });
        (
            window_id,
            self.root_view(window_id)
                .expect("should have just inserted a window and root view"),
        )
    }

    pub fn add_view<T, F>(&mut self, window_id: WindowId, build_view: F) -> ViewHandle<T>
    where
        T: View,
        F: FnOnce(&mut ViewContext<T>) -> T,
    {
        self.add_option_view(window_id, |ctx| Some(build_view(ctx)))
            .unwrap()
    }

    pub fn add_option_view<T, F>(
        &mut self,
        window_id: WindowId,
        build_view: F,
    ) -> Option<ViewHandle<T>>
    where
        T: View,
        F: FnOnce(&mut ViewContext<T>) -> Option<T>,
    {
        self.insert_view_inner(
            window_id,
            |app, view_id| {
                let mut ctx = ViewContext::new(app, window_id, view_id);
                build_view(&mut ctx).map(|view| BackendView::into_any_view(Box::new(view)))
            },
            |app, window_id, view_id| ViewHandle::new(window_id, view_id, &app.ref_counts),
        )
    }

    /// Add a view that implements the `TypedAction` trait, including the default parent view
    ///
    /// This will create the view as normal as well as register it's `handle_action` method in the
    /// typed_actions hash.
    ///
    /// Note: This is intended to be the replacement for `add_view` with the conversion to typed
    /// actions (and will subsequently be renamed to `add_view` once that is complete)
    pub(crate) fn add_typed_action_view_with_parent<V, F>(
        &mut self,
        window_id: WindowId,
        build_view: F,
        parent_view_id: EntityId,
    ) -> ViewHandle<V>
    where
        V: TypedActionView + View,
        F: FnOnce(&mut ViewContext<V>) -> V,
    {
        self.add_typed_action_view_internal(window_id, build_view, Some(parent_view_id))
    }

    /// Add a view that implements the `TypedAction` trait
    ///
    /// This will create the view as normal as well as register it's `handle_action` method in the
    /// typed_actions hash.
    ///
    /// Note: This is intended to be the replacement for `add_view` with the conversion to typed
    /// actions (and will subsequently be renamed to `add_view` once that is complete)
    pub fn add_typed_action_view<V, F>(
        &mut self,
        window_id: WindowId,
        build_view: F,
    ) -> ViewHandle<V>
    where
        V: TypedActionView + View,
        F: FnOnce(&mut ViewContext<V>) -> V,
    {
        self.add_typed_action_view_internal(window_id, build_view, None)
    }

    fn add_typed_action_view_internal<V, F>(
        &mut self,
        window_id: WindowId,
        build_view: F,
        parent_view_id: Option<EntityId>,
    ) -> ViewHandle<V>
    where
        V: TypedActionView + View,
        F: FnOnce(&mut ViewContext<V>) -> V,
    {
        self.insert_typed_action_view_inner(
            window_id,
            parent_view_id,
            |app, view_id| {
                let mut ctx = ViewContext::new(app, window_id, view_id);
                let view = build_view(&mut ctx);
                BackendView::into_any_view(Box::new(view))
            },
            |app, view_id, parent_view_id| {
                if let Some(presenter) = app.presenter(window_id) {
                    presenter.borrow_mut().set_parent(view_id, parent_view_id);
                }
            },
            |app| app.add_typed_action::<V>(),
            |app, window_id, view_id| ViewHandle::new(window_id, view_id, &app.ref_counts),
        )
    }

    pub fn open_view_tree_debug_window(&mut self, target_window_id: WindowId) {
        let Some(presenter) = self.presenter(target_window_id) else {
            return;
        };
        let Some(root_view_id) = self.root_view_id(target_window_id) else {
            return;
        };

        let Some(current_bounds) = self.window_bounds(&target_window_id) else {
            return;
        };
        let size = Vector2F::new(340., 540.);
        let origin = Vector2F::new(
            current_bounds.origin().x() + current_bounds.width() - size.x() - 20.,
            current_bounds.origin().y() + 20.,
        );

        let options = AddWindowOptions {
            window_bounds: WindowBounds::ExactPosition(RectF::new(origin, size)),
            anchor_new_windows_from_closed_position:
                NextNewWindowsHasThisWindowsBoundsUponClose::No,
            window_instance: Some("dev.warp.warpui-debug".to_owned()),
            title: Some("View Tree Debugger".to_owned()),
            ..Default::default()
        };
        self.add_window(options, |ctx| {
            crate::debug::DebugRootView::new(
                target_window_id,
                presenter.borrow().parents(),
                root_view_id,
                ctx,
            )
        });
    }

    pub fn render_view(&self, window_id: WindowId, view_id: EntityId) -> Result<Box<dyn Element>> {
        // surfacing the error of a missing window earlier
        let window = self
            .windows
            .get(&window_id)
            .ok_or_else(|| anyhow!("window not found"))?;
        window
            .views
            .get(&view_id)
            .map(|view| autotracking::render_view(window_id, view_id, || view.render(self)))
            .ok_or_else(|| anyhow!("view not found"))
    }

    pub fn render_views(&self, window_id: WindowId) -> Result<HashMap<EntityId, Box<dyn Element>>> {
        self.windows
            .get(&window_id)
            .map(|w| {
                w.views
                    .iter()
                    .map(|(id, view)| (*id, view.render(self)))
                    .collect::<HashMap<_, _>>()
            })
            .ok_or_else(|| anyhow!("window not found"))
    }
}
