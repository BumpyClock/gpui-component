use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    Styled, Window, div, prelude::FluentBuilder as _, px,
};

use gpui_component::{
    ActiveTheme, FloatingSidebar, Icon, IconName, Side, h_flex,
    sidebar::{SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    switch::Switch,
    v_flex,
};

use crate::section;

pub struct FloatingSidebarStory {
    focus_handle: FocusHandle,
    collapsed: bool,
    side: Side,
    wide_inset: bool,
}

impl FloatingSidebarStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            collapsed: false,
            side: Side::Left,
            wide_inset: false,
        }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn render_example(
        &self,
        id: &'static str,
        inset: gpui::Pixels,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let sidebar_width = px(240.0);
        let collapsed_width = px(48.0);
        let content_offset = if self.collapsed {
            collapsed_width
        } else {
            sidebar_width
        } + inset;

        let sidebar = FloatingSidebar::new(id)
            .side(self.side)
            .collapsed(self.collapsed)
            .width(sidebar_width)
            .inset(inset)
            .header_with(|collapsed, _, _cx| {
                SidebarHeader::new()
                    .child(Icon::new(IconName::PanelLeft).size_4())
                    .when(!collapsed, |this| this.child("Workspace"))
                    .when(!collapsed, |this| {
                        this.child(Icon::new(IconName::ChevronsUpDown).size_4())
                    })
            })
            .child(
                SidebarGroup::new("Navigation").child(SidebarMenu::new().children([
                    SidebarMenuItem::new("Overview").icon(IconName::LayoutDashboard),
                    SidebarMenuItem::new("Projects").icon(IconName::Folder),
                    SidebarMenuItem::new("Settings").icon(IconName::Settings2),
                ])),
            )
            .footer_with(|collapsed, _, _| {
                SidebarFooter::new()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Icon::new(IconName::CircleUser))
                            .when(!collapsed, |this| this.child("Avery")),
                    )
                    .when(!collapsed, |this| {
                        this.child(Icon::new(IconName::ChevronsUpDown).size_4())
                    })
            });

        div()
            .relative()
            .w(px(640.0))
            .h(px(360.0))
            .overflow_hidden()
            .rounded(cx.theme().radius_lg)
            .border_1()
            .border_color(cx.theme().border)
            .child(sidebar)
            .child(
                div()
                    .size_full()
                    .pt(px(16.0))
                    .when(self.side.is_left(), |this| this.pl(content_offset))
                    .when(self.side.is_right(), |this| this.pr(content_offset))
                    .child(
                        div()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().secondary)
                            .p_4()
                            .text_sm()
                            .text_color(cx.theme().secondary_foreground)
                            .child("Main content area"),
                    ),
            )
    }
}

impl super::Story for FloatingSidebarStory {
    fn title() -> &'static str {
        "Floating Sidebar"
    }

    fn description() -> &'static str {
        "FloatingSidebar combines SidebarShell and Sidebar with built-in resize handling."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for FloatingSidebarStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FloatingSidebarStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let inset = if self.wide_inset { px(12.0) } else { px(8.0) };

        v_flex().gap_6().child(
            section("Floating Sidebar").child(
                v_flex()
                    .gap_4()
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                Switch::new("floating-sidebar-collapsed")
                                    .label("Collapsed")
                                    .checked(self.collapsed)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.collapsed = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Switch::new("floating-sidebar-side")
                                    .label("Right Side")
                                    .checked(self.side.is_right())
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.side = if *checked { Side::Right } else { Side::Left };
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Switch::new("floating-sidebar-inset")
                                    .label("Wide Inset")
                                    .checked(self.wide_inset)
                                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                        this.wide_inset = *checked;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(self.render_example("floating-sidebar-example", inset, window, cx)),
            ),
        )
    }
}
