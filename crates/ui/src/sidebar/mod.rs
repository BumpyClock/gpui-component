use crate::{
    ActiveTheme, Collapsible, Icon, IconName, PixelsExt, Side, Sizable, StyledExt,
    animation::{point_to_point_animation, soft_dismiss_animation},
    button::{Button, ButtonVariants},
    global_state::GlobalState,
    h_flex,
    scroll::ScrollableElement,
    v_flex,
};
use gpui::{
    AnimationExt as _, AnyElement, App, ClickEvent, EdgesRefinement, ElementId,
    InteractiveElement as _, IntoElement, ListAlignment, ListState, ParentElement, Pixels,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, list, prelude::FluentBuilder,
    px,
};
use std::rc::Rc;
use std::time::Duration;

mod footer;
mod group;
mod header;
mod menu;
pub use footer::*;
pub use group::*;
pub use header::*;
pub use menu::*;

const DEFAULT_WIDTH: Pixels = px(255.);
const COLLAPSED_WIDTH: Pixels = px(48.);

pub trait SidebarItem: Collapsible + Clone {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement;
}

/// A Sidebar element that can contain collapsible child elements.
#[derive(IntoElement)]
pub struct Sidebar<E: SidebarItem + 'static> {
    id: ElementId,
    style: StyleRefinement,
    content: Vec<E>,
    /// header view
    header: Option<AnyElement>,
    /// footer view
    footer: Option<AnyElement>,
    /// The side of the sidebar
    side: Side,
    collapsible: bool,
    collapsed: bool,
    width: Pixels,
}

impl<E: SidebarItem> Sidebar<E> {
    /// Create a new Sidebar with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            content: vec![],
            header: None,
            footer: None,
            side: Side::Left,
            collapsible: true,
            collapsed: false,
            width: DEFAULT_WIDTH,
        }
    }

    /// Set the side of the sidebar.
    ///
    /// Default is `Side::Left`.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Set the sidebar to be collapsible, default is true
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        self.collapsible = collapsible;
        self
    }

    /// Set the sidebar to be collapsed
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Set the expanded width of the sidebar.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = width.into();
        self
    }

    /// Set the header of the sidebar.
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    /// Set the footer of the sidebar.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Add a child element to the sidebar, the child must implement `Collapsible`
    pub fn child(mut self, child: E) -> Self {
        self.content.push(child);
        self
    }

    /// Add multiple children to the sidebar, the children must implement `Collapsible`
    pub fn children(mut self, children: impl IntoIterator<Item = E>) -> Self {
        self.content.extend(children);
        self
    }
}

/// Toggle button to collapse/expand the [`Sidebar`].
#[derive(IntoElement)]
pub struct SidebarToggleButton {
    btn: Button,
    collapsed: bool,
    side: Side,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl SidebarToggleButton {
    /// Create a new SidebarToggleButton.
    pub fn new() -> Self {
        Self {
            btn: Button::new("collapse").ghost().small(),
            collapsed: false,
            side: Side::Left,
            on_click: None,
        }
    }

    /// Set the side of the toggle button.
    ///
    /// Default is `Side::Left`.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Set the collapsed state of the toggle button.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Add a click handler to the toggle button.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }
}

impl RenderOnce for SidebarToggleButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let collapsed = self.collapsed;
        let on_click = self.on_click.clone();

        let icon = if collapsed {
            if self.side.is_left() {
                IconName::PanelLeftOpen
            } else {
                IconName::PanelRightOpen
            }
        } else {
            if self.side.is_left() {
                IconName::PanelLeftClose
            } else {
                IconName::PanelRightClose
            }
        };

        self.btn
            .when_some(on_click, |this, on_click| {
                this.on_click(move |ev, window, cx| {
                    on_click(ev, window, cx);
                })
            })
            .icon(Icon::new(icon).size_4())
    }
}

impl<E: SidebarItem> Styled for Sidebar<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<E: SidebarItem> RenderOnce for Sidebar<E> {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.style.padding = EdgesRefinement::default();

        let reduced_motion = GlobalState::global(cx).reduced_motion();
        let motion = cx.theme().motion.clone();
        let collapse_duration = Duration::from_millis(u64::from(motion.soft_dismiss_duration_ms));
        let target_collapsed = self.collapsed;
        let sidebar_id = self.id.clone();
        let expanded_width = self.width;

        let width_state = window.use_keyed_state(
            SharedString::from(format!("{}-collapsed-target", sidebar_id)),
            cx,
            |_, _| target_collapsed,
        );
        let last_collapsed_target = *width_state.read(cx);
        let collapse_changed = last_collapsed_target != target_collapsed;
        if collapse_changed {
            width_state.update(cx, |state, _| *state = target_collapsed);
        }

        let visual_collapse_state = window.use_keyed_state(
            SharedString::from(format!("{}-collapsed-visual", sidebar_id)),
            cx,
            |_, _| target_collapsed,
        );
        if collapse_changed {
            if reduced_motion {
                visual_collapse_state.update(cx, |state, _| *state = target_collapsed);
            } else if target_collapsed {
                cx.spawn({
                    let visual_collapse_state = visual_collapse_state.clone();
                    let width_state = width_state.clone();
                    async move |cx| {
                        cx.background_executor().timer(collapse_duration).await;
                        let still_collapsed = width_state.update(cx, |state, _| *state);
                        if still_collapsed {
                            _ = visual_collapse_state.update(cx, |state, _| *state = true);
                        }
                    }
                })
                .detach();
            } else {
                visual_collapse_state.update(cx, |state, _| *state = false);
            }
        }
        let visual_collapsed = *visual_collapse_state.read(cx);

        let content_len = self.content.len();
        let overdraw = px(window.viewport_size().height.as_f32() * 0.3);
        let list_state = window
            .use_keyed_state(
                SharedString::from(format!("{}-list-state", sidebar_id)),
                cx,
                |_, _| ListState::new(content_len, ListAlignment::Top, overdraw),
            )
            .read(cx)
            .clone();
        if list_state.item_count() != content_len {
            list_state.reset(content_len);
        }

        let item_id_prefix = sidebar_id.clone();
        let sidebar = v_flex()
            .id(sidebar_id.clone())
            .w(expanded_width)
            .flex_shrink_0()
            .h_full()
            .overflow_hidden()
            .relative()
            .bg(cx.theme().sidebar)
            .text_color(cx.theme().sidebar_foreground)
            .border_color(cx.theme().sidebar_border)
            .map(|this| match self.side {
                Side::Left => this.border_r_1(),
                Side::Right => this.border_l_1(),
            })
            .refine_style(&self.style)
            .when(target_collapsed, |this| this.w(COLLAPSED_WIDTH))
            .when(visual_collapsed, |this| this.gap_2())
            .when_some(self.header.take(), |this, header| {
                this.child(
                    h_flex()
                        .id("header")
                        .pt_3()
                        .px_3()
                        .gap_2()
                        .when(visual_collapsed, |this| this.pt_2().px_2())
                        .child(header),
                )
            })
            .child(
                v_flex().id("content").flex_1().min_h_0().child(
                    v_flex()
                        .id("inner")
                        .size_full()
                        .px_3()
                        .gap_y_3()
                        .when(visual_collapsed, |this| this.p_2())
                        .child(
                            list(list_state.clone(), {
                                move |ix, window, cx| {
                                    let group = self.content.get(ix).cloned();
                                    let is_first = ix == 0;
                                    let is_last =
                                        content_len > 0 && ix == content_len.saturating_sub(1);
                                    div()
                                        .id(ix)
                                        .when_some(group, |this, group| {
                                            this.child(
                                                group
                                                    .collapsed(visual_collapsed)
                                                    .render(
                                                        SharedString::from(format!(
                                                            "{}-{}",
                                                            item_id_prefix, ix
                                                        )),
                                                        window,
                                                        cx,
                                                    )
                                                    .into_any_element(),
                                            )
                                        })
                                        .when(is_first, |this| this.pt_3())
                                        .when(is_last, |this| this.pb_3())
                                        .into_any_element()
                                }
                            })
                            .size_full(),
                        )
                        .vertical_scrollbar(&list_state),
                ),
            )
            .when_some(self.footer.take(), |this, footer| {
                this.child(
                    h_flex()
                        .id("footer")
                        .pb_3()
                        .px_3()
                        .gap_2()
                        .when(visual_collapsed, |this| this.pt_2().px_2())
                        .child(footer),
                )
            });

        if reduced_motion || !collapse_changed {
            sidebar.into_any_element()
        } else {
            let collapsed_width = COLLAPSED_WIDTH;
            let from_width = if target_collapsed {
                expanded_width
            } else {
                collapsed_width
            };
            let to_width = if target_collapsed {
                collapsed_width
            } else {
                expanded_width
            };
            let anim = if target_collapsed {
                soft_dismiss_animation(&motion, reduced_motion)
            } else {
                point_to_point_animation(&motion, reduced_motion)
            };

            if let Some(anim) = anim {
                sidebar
                    .with_animation(
                        SharedString::from(format!(
                            "{}-sidebar-width-{}",
                            sidebar_id,
                            u8::from(target_collapsed)
                        )),
                        anim,
                        move |this, delta| this.w(from_width + (to_width - from_width) * delta),
                    )
                    .into_any_element()
            } else {
                sidebar.into_any_element()
            }
        }
    }
}
