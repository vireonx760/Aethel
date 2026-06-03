use crate::gui::command::{CommandId, CommandPayload, UiCommand, WidgetId};
use crate::gui::geometry::{BoxConstraints, Point, Size};
use crate::gui::style::Theme;
use crate::gui::widget::{GuiManager, Widget};
use crate::widgets::{Button, Checkbox, Label, Panel, ProgressBar, Separator, Slider, TextInput};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::RangeInclusive;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Response {
    pub id: WidgetId,
    pub hovered: bool,
    pub active: bool,
    pub focused: bool,
    pub clicked: bool,
    pub changed: bool,
}

impl Response {
    #[inline]
    pub const fn new(id: WidgetId) -> Self {
        Self {
            id,
            hovered: false,
            active: false,
            focused: false,
            clicked: false,
            changed: false,
        }
    }

    #[inline]
    pub fn clicked(self) -> bool {
        self.clicked
    }

    #[inline]
    pub fn hovered(self) -> bool {
        self.hovered
    }

    #[inline]
    pub fn active(self) -> bool {
        self.active
    }

    #[inline]
    pub fn focused(self) -> bool {
        self.focused
    }

    #[inline]
    pub fn changed(self) -> bool {
        self.changed
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiDiagnostics {
    pub duplicate_keys: Vec<WidgetId>,
    pub type_mismatches: Vec<WidgetId>,
}

impl UiDiagnostics {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.duplicate_keys.is_empty() && self.type_mismatches.is_empty()
    }
}

pub struct UiState {
    retained: HashMap<WidgetId, Box<dyn Widget>>,
    order: Vec<WidgetId>,
    diagnostics: UiDiagnostics,
    theme: Theme,
    frame: u64,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            retained: HashMap::new(),
            order: Vec::new(),
            diagnostics: UiDiagnostics::default(),
            theme: Theme::dark(),
            frame: 0,
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn diagnostics(&self) -> &UiDiagnostics {
        &self.diagnostics
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }

    fn begin_frame(&mut self, gui: &mut GuiManager) {
        self.retained.clear();
        let widgets = gui.take_widgets_for_rebuild();
        for (id, widget) in self.order.drain(..).zip(widgets) {
            self.retained.insert(id, widget);
        }
        self.diagnostics = UiDiagnostics::default();
        self.frame = self.frame.wrapping_add(1);
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiLayout {
    pub spacing: f32,
    pub padding: f32,
    pub default_width: f32,
}

impl Default for UiLayout {
    fn default() -> Self {
        Self {
            spacing: 10.0,
            padding: 16.0,
            default_width: 240.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
struct LayoutScope {
    cursor: [f32; 2],
    start: [f32; 2],
    axis: UiAxis,
    spacing: f32,
    used: [f32; 2],
}

impl LayoutScope {
    fn new(start: [f32; 2], axis: UiAxis, spacing: f32) -> Self {
        Self {
            cursor: start,
            start,
            axis,
            spacing,
            used: [0.0, 0.0],
        }
    }
}

#[derive(Debug)]
struct PanelGroup {
    panel: WidgetId,
    children: Vec<WidgetId>,
}

pub struct Ui<'a> {
    state: &'a mut UiState,
    responses: HashMap<WidgetId, Vec<CommandPayload>>,
    widgets: Vec<Box<dyn Widget>>,
    order: Vec<WidgetId>,
    used_ids: HashSet<WidgetId>,
    id_stack: Vec<u64>,
    scopes: Vec<LayoutScope>,
    groups: Vec<PanelGroup>,
    active_panel_children: Vec<Vec<WidgetId>>,
    duplicate_serial: u64,
    layout: UiLayout,
}

impl<'a> Ui<'a> {
    pub fn rebuild(
        gui: &mut GuiManager,
        state: &'a mut UiState,
        commands: &[UiCommand],
        mut build: impl FnMut(&mut Ui<'_>),
    ) {
        state.begin_frame(gui);
        let mut ui = Self::new(state, commands);
        build(&mut ui);
        ui.finish(gui);
    }

    fn new(state: &'a mut UiState, commands: &[UiCommand]) -> Self {
        let mut responses: HashMap<WidgetId, Vec<CommandPayload>> = HashMap::new();
        for command in commands {
            let id = command
                .widget
                .unwrap_or_else(|| WidgetId(command.id.raw() as usize));
            responses
                .entry(id)
                .or_default()
                .push(command.payload.clone());
        }

        Self {
            state,
            responses,
            widgets: Vec::new(),
            order: Vec::new(),
            used_ids: HashSet::new(),
            id_stack: vec![0xcbf2_9ce4_8422_2325],
            scopes: vec![LayoutScope::new([24.0, 24.0], UiAxis::Vertical, 10.0)],
            groups: Vec::new(),
            active_panel_children: Vec::new(),
            duplicate_serial: 0,
            layout: UiLayout::default(),
        }
    }

    fn finish(self, gui: &mut GuiManager) {
        let id_to_index: HashMap<WidgetId, usize> = self
            .order
            .iter()
            .copied()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect();

        gui.replace_widgets_for_rebuild(self.widgets);
        for group in self.groups {
            let Some(&panel_idx) = id_to_index.get(&group.panel) else {
                continue;
            };
            let children = group
                .children
                .iter()
                .filter_map(|id| id_to_index.get(id).copied())
                .collect::<Vec<_>>();
            if !children.is_empty() {
                gui.register_clip_group(panel_idx, children);
            } else {
                gui.register_panel(panel_idx);
            }
        }
        self.state.order = self.order;
    }

    pub fn set_layout(&mut self, layout: UiLayout) {
        self.layout = layout;
        if let Some(scope) = self.scopes.last_mut() {
            scope.spacing = layout.spacing;
        }
    }

    pub fn key(&mut self, key: impl AsRef<str>, build: impl FnOnce(&mut Ui<'_>)) {
        self.with_id(key, build);
    }

    pub fn with_id(&mut self, key: impl AsRef<str>, build: impl FnOnce(&mut Ui<'_>)) {
        let parent = self.current_parent_hash();
        let id_hash = hash_parts(parent, "scope", key.as_ref());
        self.id_stack.push(id_hash);
        build(self);
        self.id_stack.pop();
    }

    pub fn column(&mut self, build: impl FnOnce(&mut Ui<'_>)) {
        self.scoped_layout(UiAxis::Vertical, build);
    }

    pub fn row(&mut self, build: impl FnOnce(&mut Ui<'_>)) {
        self.scoped_layout(UiAxis::Horizontal, build);
    }

    pub fn panel(&mut self, key: impl AsRef<str>, build: impl FnOnce(&mut Ui<'_>)) -> Response {
        self.panel_with(key, [360.0, 240.0], build)
    }

    pub fn panel_with(
        &mut self,
        key: impl AsRef<str>,
        size: [f32; 2],
        build: impl FnOnce(&mut Ui<'_>),
    ) -> Response {
        let key = key.as_ref();
        let id = self.reserve_id("panel", key);
        let pos = self.allocate(size);
        let panel_fill = self.state.theme.panel.fill.to_array();
        let panel_radius = self.state.theme.panel.radius.uniform_for_shader();
        let mut response = Response::new(id);
        let mut widget = self.take_or_create(id, || {
            Box::new(Panel::new(pos, size).color(panel_fill).radius(panel_radius))
        });

        if let Some(panel) = widget.as_any_mut().downcast_mut::<Panel>() {
            response.active = panel.requests_repaint();
            panel.color = panel_fill;
            panel.radius = panel_radius;
            panel.layout(BoxConstraints::tight(Size::new(size[0], size[1])));
            panel.set_position(Point::new(pos[0], pos[1]));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            widget = Box::new(Panel::new(pos, size).color(panel_fill).radius(panel_radius));
        }

        self.push_widget(id, widget);

        let start = [pos[0] + self.layout.padding, pos[1] + self.layout.padding];
        self.active_panel_children.push(Vec::new());
        self.id_stack
            .push(hash_parts(self.current_parent_hash(), "panel", key));
        self.scopes.push(LayoutScope::new(
            start,
            UiAxis::Vertical,
            self.layout.spacing,
        ));
        build(self);
        self.scopes.pop();
        self.id_stack.pop();
        let children = self.active_panel_children.pop().unwrap_or_default();
        self.groups.push(PanelGroup {
            panel: id,
            children,
        });
        response
    }

    pub fn label(&mut self, text: impl AsRef<str>) -> Response {
        self.label_styled(
            text,
            self.state.theme.text.size,
            self.state.theme.text.color.to_array(),
        )
    }

    pub fn label_styled(&mut self, text: impl AsRef<str>, size: f32, color: [f32; 4]) -> Response {
        let text = text.as_ref();
        let id = self.reserve_id("label", text);
        let height = size.max(1.0) * 1.2 + 4.0;
        let width = estimate_text_width(text, size).max(1.0);
        let pos = self.allocate([width, height]);
        let mut widget = self.take_or_create(id, || {
            Box::new(Label::new(pos, text).scale(size).color(color))
        });

        if let Some(label) = widget.as_any_mut().downcast_mut::<Label>() {
            label.text.clear();
            label.text.push_str(text);
            label.scale = size;
            label.color = color;
            label.layout(BoxConstraints::loose(Size::new(width, height)));
            label.set_position(Point::new(pos[0], pos[1]));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            widget = Box::new(Label::new(pos, text).scale(size).color(color));
        }

        self.push_widget(id, widget);
        Response::new(id)
    }

    pub fn button(&mut self, label: impl AsRef<str>) -> Response {
        self.button_sized(label, [160.0, 38.0])
    }

    pub fn button_sized(&mut self, label: impl AsRef<str>, size: [f32; 2]) -> Response {
        let label = label.as_ref();
        let id = self.reserve_id("button", label);
        let pos = self.allocate(size);
        let command = command_for::<()>(id);
        let mut response = self.base_response(id);
        let mut widget = self.take_or_create(id, || {
            Box::new(Button::new(pos, size, label).on_click_cmd(command))
        });

        if let Some(button) = widget.as_any_mut().downcast_mut::<Button>() {
            response.hovered = button.hovered;
            response.active = button.pressed;
            button.text.clear();
            button.text.push_str(label);
            button.size = size;
            button.base_color = self.state.theme.button.normal.fill.to_array();
            button.hover_color = self.state.theme.button.hovered.fill.to_array();
            button.pressed_color = self.state.theme.button.pressed.fill.to_array();
            button.set_position(Point::new(pos[0], pos[1]));
            button.layout(BoxConstraints::tight(Size::new(size[0], size[1])));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            widget = Box::new(Button::new(pos, size, label).on_click_cmd(command));
        }

        self.push_widget(id, widget);
        response
    }

    pub fn checkbox(&mut self, label: impl AsRef<str>, value: &mut bool) -> Response {
        let label = label.as_ref();
        let id = self.reserve_id("checkbox", label);
        let width = 34.0 + estimate_text_width(label, 16.0);
        let pos = self.allocate([width, 28.0]);
        let command = command_for::<bool>(id);
        let response = self.base_response(id);
        let mut widget = self.take_or_create(id, || {
            let mut checkbox = Checkbox::new(pos).with_label(label).on_change_cmd(command);
            checkbox.checked = *value;
            Box::new(checkbox)
        });

        if let Some(checkbox) = widget.as_any_mut().downcast_mut::<Checkbox>() {
            if response.changed {
                *value = checkbox.checked;
            }
            checkbox.label.clear();
            checkbox.label.push_str(label);
            checkbox.checked = *value;
            checkbox.set_position(Point::new(pos[0], pos[1]));
            checkbox.layout(BoxConstraints::loose(Size::new(width, 28.0)));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            let mut checkbox = Checkbox::new(pos).with_label(label).on_change_cmd(command);
            checkbox.checked = *value;
            widget = Box::new(checkbox);
        }

        self.push_widget(id, widget);
        response
    }

    pub fn slider(
        &mut self,
        label: impl AsRef<str>,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> Response {
        let label = label.as_ref();
        let id = self.reserve_id("slider", label);
        self.label_styled(
            label,
            self.state.theme.muted_text.size,
            self.state.theme.muted_text.color.to_array(),
        );
        let length = self.layout.default_width;
        let pos = self.allocate([length, 36.0]);
        let command = command_for::<f32>(id);
        let mut response = self.base_response(id);
        let (min, max) = range_bounds(range);
        let mut widget = self.take_or_create(id, || {
            let mut slider = Slider::new(pos, length).on_change_cmd(command);
            slider.value = normalize_range(*value, min, max);
            Box::new(slider)
        });

        if let Some(slider) = widget.as_any_mut().downcast_mut::<Slider>() {
            response.active = slider.dragging;
            if response.changed {
                *value = denormalize_range(slider.value, min, max);
            }
            *value = value.clamp(min, max);
            slider.value = normalize_range(*value, min, max);
            slider.length = length;
            slider.set_position(Point::new(pos[0], pos[1]));
            slider.layout(BoxConstraints::tight(Size::new(length, 36.0)));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            let mut slider = Slider::new(pos, length).on_change_cmd(command);
            slider.value = normalize_range(*value, min, max);
            widget = Box::new(slider);
        }

        self.push_widget(id, widget);
        response
    }

    pub fn text_input(&mut self, key: impl AsRef<str>, value: &mut String) -> Response {
        let key = key.as_ref();
        let id = self.reserve_id("text_input", key);
        let size = [self.layout.default_width, 40.0];
        let pos = self.allocate(size);
        let command = command_for::<String>(id);
        let mut response = self.base_response(id);
        let text_payload = self.text_payload(id).map(str::to_owned);
        let mut widget = self.take_or_create(id, || {
            Box::new(
                TextInput::new(pos, size, key)
                    .initial_text(value.as_str())
                    .on_change_cmd(command),
            )
        });

        if let Some(input) = widget.as_any_mut().downcast_mut::<TextInput>() {
            response.focused = input.focused;
            if let Some(text) = text_payload {
                *value = text;
            }
            input.placeholder.clear();
            input.placeholder.push_str(key);
            if !input.focused && input.get_text() != value {
                input.set_text(value.as_str());
            }
            input.size = size;
            input.set_position(Point::new(pos[0], pos[1]));
            input.layout(BoxConstraints::tight(Size::new(size[0], size[1])));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            widget = Box::new(
                TextInput::new(pos, size, key)
                    .initial_text(value.as_str())
                    .on_change_cmd(command),
            );
        }

        self.push_widget(id, widget);
        response
    }

    pub fn progress_bar(&mut self, key: impl AsRef<str>, progress: f32) -> Response {
        let key = key.as_ref();
        let id = self.reserve_id("progress_bar", key);
        let size = [self.layout.default_width, 20.0];
        let pos = self.allocate(size);
        let mut widget = self.take_or_create(id, || Box::new(ProgressBar::new(pos, size)));
        if let Some(bar) = widget.as_any_mut().downcast_mut::<ProgressBar>() {
            bar.set_progress(progress);
            bar.set_position(Point::new(pos[0], pos[1]));
            bar.layout(BoxConstraints::tight(Size::new(size[0], size[1])));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            let mut bar = ProgressBar::new(pos, size);
            bar.set_progress(progress);
            widget = Box::new(bar);
        }
        self.push_widget(id, widget);
        Response::new(id)
    }

    pub fn separator(&mut self) -> Response {
        let id = self.reserve_id("separator", "line");
        let size = [self.layout.default_width, 1.0];
        let pos = self.allocate([size[0], self.layout.spacing.max(1.0)]);
        let mut widget = self.take_or_create(id, || Box::new(Separator::new(pos, size)));
        if let Some(separator) = widget.as_any_mut().downcast_mut::<Separator>() {
            separator.set_position(Point::new(pos[0], pos[1]));
            separator.layout(BoxConstraints::tight(Size::new(size[0], size[1])));
        } else {
            self.state.diagnostics.type_mismatches.push(id);
            widget = Box::new(Separator::new(pos, size));
        }
        self.push_widget(id, widget);
        Response::new(id)
    }

    pub fn diagnostics(&self) -> &UiDiagnostics {
        &self.state.diagnostics
    }

    fn scoped_layout(&mut self, axis: UiAxis, build: impl FnOnce(&mut Ui<'_>)) {
        let start = self.current_scope().cursor;
        self.scopes
            .push(LayoutScope::new(start, axis, self.layout.spacing));
        build(self);
        let scope = self
            .scopes
            .pop()
            .unwrap_or_else(|| LayoutScope::new(start, axis, self.layout.spacing));
        self.advance_parent(scope.used);
    }

    fn reserve_id(&mut self, kind: &str, key: &str) -> WidgetId {
        let mut id = WidgetId(hash_parts(self.current_parent_hash(), kind, key) as usize);
        if id.0 == 0 {
            id.0 = 1;
        }
        if self.used_ids.insert(id) {
            return id;
        }

        self.state.diagnostics.duplicate_keys.push(id);
        loop {
            self.duplicate_serial = self.duplicate_serial.wrapping_add(1);
            let duplicate_key = format!("{key}#{}", self.duplicate_serial);
            let mut candidate =
                WidgetId(hash_parts(self.current_parent_hash(), kind, &duplicate_key) as usize);
            if candidate.0 == 0 {
                candidate.0 = 1;
            }
            if self.used_ids.insert(candidate) {
                return candidate;
            }
        }
    }

    fn take_or_create(
        &mut self,
        id: WidgetId,
        create: impl FnOnce() -> Box<dyn Widget>,
    ) -> Box<dyn Widget> {
        self.state.retained.remove(&id).unwrap_or_else(create)
    }

    fn push_widget(&mut self, id: WidgetId, widget: Box<dyn Widget>) {
        if let Some(children) = self.active_panel_children.last_mut() {
            children.push(id);
        }
        self.order.push(id);
        self.widgets.push(widget);
    }

    fn base_response(&self, id: WidgetId) -> Response {
        let mut response = Response::new(id);
        if let Some(payloads) = self.responses.get(&id) {
            response.clicked = payloads.contains(&CommandPayload::None);
            response.changed = payloads
                .iter()
                .any(|payload| *payload != CommandPayload::None);
        }
        response
    }

    fn text_payload(&self, id: WidgetId) -> Option<&str> {
        self.responses.get(&id).and_then(|payloads| {
            payloads.iter().find_map(|payload| match payload {
                CommandPayload::Text(text) => Some(text.as_str()),
                _ => None,
            })
        })
    }

    fn allocate(&mut self, size: [f32; 2]) -> [f32; 2] {
        let scope = self.current_scope_mut();
        let pos = scope.cursor;
        match scope.axis {
            UiAxis::Horizontal => {
                scope.cursor[0] += size[0] + scope.spacing;
                scope.used[0] =
                    (scope.cursor[0] - scope.start[0] - scope.spacing).max(scope.used[0]);
                scope.used[1] = scope.used[1].max(size[1]);
            }
            UiAxis::Vertical => {
                scope.cursor[1] += size[1] + scope.spacing;
                scope.used[0] = scope.used[0].max(size[0]);
                scope.used[1] =
                    (scope.cursor[1] - scope.start[1] - scope.spacing).max(scope.used[1]);
            }
        }
        pos
    }

    fn advance_parent(&mut self, size: [f32; 2]) {
        let scope = self.current_scope_mut();
        match scope.axis {
            UiAxis::Horizontal => {
                scope.cursor[0] += size[0] + scope.spacing;
                scope.used[0] =
                    (scope.cursor[0] - scope.start[0] - scope.spacing).max(scope.used[0]);
                scope.used[1] = scope.used[1].max(size[1]);
            }
            UiAxis::Vertical => {
                scope.cursor[1] += size[1] + scope.spacing;
                scope.used[0] = scope.used[0].max(size[0]);
                scope.used[1] =
                    (scope.cursor[1] - scope.start[1] - scope.spacing).max(scope.used[1]);
            }
        }
    }

    fn current_parent_hash(&self) -> u64 {
        self.id_stack.last().copied().unwrap_or(0)
    }

    fn current_scope(&self) -> &LayoutScope {
        let index = self.scopes.len().saturating_sub(1);
        &self.scopes[index]
    }

    fn current_scope_mut(&mut self) -> &mut LayoutScope {
        let index = self.scopes.len().saturating_sub(1);
        &mut self.scopes[index]
    }
}

#[inline]
fn command_for<T>(id: WidgetId) -> CommandId<T> {
    CommandId::from_raw(id.0 as u64)
}

fn hash_parts(parent: u64, kind: &str, key: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    parent.hash(&mut hasher);
    kind.hash(&mut hasher);
    key.hash(&mut hasher);
    hasher.finish()
}

fn estimate_text_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size.max(1.0) * 0.62 + 4.0
}

fn range_bounds(range: RangeInclusive<f32>) -> (f32, f32) {
    let start = *range.start();
    let end = *range.end();
    if start <= end {
        (start, end)
    } else {
        (end, start)
    }
}

fn normalize_range(value: f32, min: f32, max: f32) -> f32 {
    let span = max - min;
    if span <= f32::EPSILON {
        0.0
    } else {
        ((value - min) / span).clamp(0.0, 1.0)
    }
}

fn denormalize_range(value: f32, min: f32, max: f32) -> f32 {
    min + value.clamp(0.0, 1.0) * (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_produces_stable_widget_id_across_rebuilds() {
        let mut gui = GuiManager::new();
        let mut state = UiState::new();
        let mut first = None;
        Ui::rebuild(&mut gui, &mut state, &[], |ui| {
            first = Some(ui.button("Apply").id);
        });
        let mut second = None;
        Ui::rebuild(&mut gui, &mut state, &[], |ui| {
            second = Some(ui.button("Apply").id);
        });

        assert_eq!(first, second);
    }

    #[test]
    fn text_input_state_survives_rebuild() {
        let mut gui = GuiManager::new();
        let mut state = UiState::new();
        let mut name = String::from("Ada");
        Ui::rebuild(&mut gui, &mut state, &[], |ui| {
            ui.text_input("name", &mut name);
        });

        let Some(input) = gui.widgets[0].as_any_mut().downcast_mut::<TextInput>() else {
            unreachable!("text input should be retained as the first widget");
        };
        input.focus();

        Ui::rebuild(&mut gui, &mut state, &[], |ui| {
            ui.text_input("name", &mut name);
        });

        let Some(input) = gui.widgets[0].as_any().downcast_ref::<TextInput>() else {
            unreachable!("text input should still be retained");
        };
        assert!(input.focused);
    }

    #[test]
    fn duplicate_keys_are_reported() {
        let mut gui = GuiManager::new();
        let mut state = UiState::new();
        Ui::rebuild(&mut gui, &mut state, &[], |ui| {
            ui.button("Apply");
            ui.button("Apply");
        });

        assert!(!state.diagnostics().duplicate_keys.is_empty());
    }

    #[test]
    fn button_command_maps_to_clicked_response() {
        let mut gui = GuiManager::new();
        let mut state = UiState::new();
        let mut id = WidgetId(0);
        Ui::rebuild(&mut gui, &mut state, &[], |ui| {
            id = ui.button("Apply").id;
        });

        let commands = vec![UiCommand::new(command_for::<()>(id))];
        let mut clicked = false;
        Ui::rebuild(&mut gui, &mut state, &commands, |ui| {
            clicked = ui.button("Apply").clicked();
        });

        assert!(clicked);
    }
}
