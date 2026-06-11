use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use tui_textarea::TextArea;

use crate::model::Priority;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FormInput {
    pub description: String,
    pub project: String,
    pub priority: Option<Priority>,
    pub due: String,
    pub tags: String,
    pub selected_deps: Vec<usize>,
    pub selected_files: Vec<usize>,
}

/// Input to the form: existing data + available choices.
pub struct FormContext {
    pub initial: FormInput,
    pub available_deps: Vec<(String, String)>, // (display_id, description)
    pub available_files: Vec<String>,
    /// Which deps are "suggested" by LLM (shown dim until user acts)
    pub suggested_dep_indices: Vec<usize>,
    /// Which files are "suggested" by LLM
    pub suggested_file_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    Description,
    Project,
    Priority,
    Due,
    Tags,
    Dependencies,
    Files,
    Submit,
    Cancel,
}

const ALL_FIELDS: &[Focus] = &[
    Focus::Description,
    Focus::Project,
    Focus::Priority,
    Focus::Due,
    Focus::Tags,
    Focus::Dependencies,
    Focus::Files,
    Focus::Submit,
    Focus::Cancel,
];

struct FormState<'a> {
    focus: Focus,
    desc_area: TextArea<'a>,
    project_area: TextArea<'a>,
    due_area: TextArea<'a>,
    tags_area: TextArea<'a>,
    priority: Option<Priority>,
    dep_state: ListState,
    file_state: ListState,
    selected_deps: Vec<bool>,
    selected_files: Vec<bool>,
    ctx: FormContext,
    submitted: bool,
    cancelled: bool,
    due_error: bool,
    due_preset_idx: usize,
}

impl<'a> FormState<'a> {
    fn new(ctx: FormContext) -> Self {
        let mut desc_area = TextArea::default();
        desc_area.insert_str(&ctx.initial.description);

        let mut project_area = TextArea::default();
        project_area.insert_str(&ctx.initial.project);

        let mut due_area = TextArea::default();
        due_area.insert_str(&ctx.initial.due);

        let mut tags_area = TextArea::default();
        tags_area.insert_str(&ctx.initial.tags);

        let n_deps = ctx.available_deps.len();
        let n_files = ctx.available_files.len();

        let mut selected_deps = vec![false; n_deps];
        for &i in &ctx.initial.selected_deps {
            if i < n_deps {
                selected_deps[i] = true;
            }
        }
        let mut selected_files = vec![false; n_files];
        for &i in &ctx.initial.selected_files {
            if i < n_files {
                selected_files[i] = true;
            }
        }

        let mut dep_state = ListState::default();
        if n_deps > 0 {
            dep_state.select(Some(0));
        }
        let mut file_state = ListState::default();
        if n_files > 0 {
            file_state.select(Some(0));
        }

        FormState {
            focus: Focus::Description,
            desc_area,
            project_area,
            due_area,
            tags_area,
            priority: ctx.initial.priority.clone(),
            dep_state,
            file_state,
            selected_deps,
            selected_files,
            ctx,
            submitted: false,
            cancelled: false,
            due_error: false,
            due_preset_idx: 0,
        }
    }

    fn next_focus(&mut self) {
        let idx = ALL_FIELDS.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = ALL_FIELDS[(idx + 1) % ALL_FIELDS.len()];
    }

    fn prev_focus(&mut self) {
        let idx = ALL_FIELDS.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = ALL_FIELDS[(idx + ALL_FIELDS.len() - 1) % ALL_FIELDS.len()];
    }

    fn toggle_dep(&mut self) {
        if let Some(i) = self.dep_state.selected() {
            if i < self.selected_deps.len() {
                self.selected_deps[i] = !self.selected_deps[i];
            }
        }
    }

    fn toggle_file(&mut self) {
        if let Some(i) = self.file_state.selected() {
            if i < self.selected_files.len() {
                self.selected_files[i] = !self.selected_files[i];
            }
        }
    }

    fn cycle_priority(&mut self, forward: bool) {
        self.priority = match (&self.priority, forward) {
            (None, true) => Some(Priority::L),
            (Some(Priority::L), true) => Some(Priority::M),
            (Some(Priority::M), true) => Some(Priority::H),
            (Some(Priority::H), true) => None,
            (None, false) => Some(Priority::H),
            (Some(Priority::H), false) => Some(Priority::M),
            (Some(Priority::M), false) => Some(Priority::L),
            (Some(Priority::L), false) => None,
        };
    }

    fn validate_due(&mut self) {
        let text = self.due_area.lines().join("");
        self.due_error = !crate::dates::is_valid_due(&text);
    }

    fn set_due_text(&mut self, value: &str) {
        let mut ta = TextArea::default();
        ta.insert_str(value);
        self.due_area = ta;
        self.validate_due();
    }

    fn cycle_due(&mut self, forward: bool) {
        let presets = crate::dates::DUE_PRESETS;
        // Find the current preset index if the text matches one, else start fresh
        let current = self.due_area.lines().join("");
        let cur_idx = presets
            .iter()
            .position(|p| *p == current.trim())
            .unwrap_or(0);
        let len = presets.len();
        let next = if forward {
            (cur_idx + 1) % len
        } else {
            (cur_idx + len - 1) % len
        };
        self.due_preset_idx = next;
        let value = presets[next].to_string();
        self.set_due_text(&value);
    }

    fn can_submit(&self) -> bool {
        !self.desc_area.lines().join("").trim().is_empty() && !self.due_error
    }

    fn collect_result(&self) -> FormInput {
        let dep_indices = self
            .selected_deps
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| i)
            .collect();
        let file_indices = self
            .selected_files
            .iter()
            .enumerate()
            .filter(|(_, v)| **v)
            .map(|(i, _)| i)
            .collect();
        FormInput {
            description: self.desc_area.lines().join(""),
            project: self.project_area.lines().join(""),
            priority: self.priority.clone(),
            due: self.due_area.lines().join(""),
            tags: self.tags_area.lines().join(""),
            selected_deps: dep_indices,
            selected_files: file_indices,
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Run the form. Returns Some(FormInput) on submit, None on cancel.
pub fn run_form<B: Backend>(
    terminal: &mut Terminal<B>,
    ctx: FormContext,
) -> Result<Option<FormInput>> {
    let mut state = FormState::new(ctx);

    loop {
        terminal.draw(|f| render(f, &mut state))?;

        if let Event::Key(key) = event::read()? {
            // Many terminals emit both Press and Release events; only act on Press
            // (and Repeat) to avoid every interaction firing twice.
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => {
                    state.cancelled = true;
                }
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    if state.can_submit() {
                        state.submitted = true;
                    }
                }
                (KeyCode::Tab, _) => state.next_focus(),
                (KeyCode::BackTab, _) => state.prev_focus(),
                (KeyCode::Enter, _) => {
                    match state.focus {
                        Focus::Submit => {
                            if state.can_submit() {
                                state.submitted = true;
                            }
                        }
                        Focus::Cancel => {
                            state.cancelled = true;
                        }
                        Focus::Dependencies => state.toggle_dep(),
                        Focus::Files => state.toggle_file(),
                        _ => state.next_focus(),
                    }
                }
                (KeyCode::Char(' '), _) => {
                    match state.focus {
                        Focus::Dependencies => state.toggle_dep(),
                        Focus::Files => state.toggle_file(),
                        _ => {}
                    }
                }
                (KeyCode::Left, _) if state.focus == Focus::Priority => {
                    state.cycle_priority(false);
                }
                (KeyCode::Right, _) if state.focus == Focus::Priority => {
                    state.cycle_priority(true);
                }
                (KeyCode::Left, _) if state.focus == Focus::Due => {
                    state.cycle_due(false);
                }
                (KeyCode::Right, _) if state.focus == Focus::Due => {
                    state.cycle_due(true);
                }
                (KeyCode::Up, _) => match state.focus {
                    Focus::Dependencies => {
                        let len = state.ctx.available_deps.len();
                        if len > 0 {
                            let cur = state.dep_state.selected().unwrap_or(0);
                            state.dep_state.select(Some(cur.saturating_sub(1)));
                        }
                    }
                    Focus::Files => {
                        let len = state.ctx.available_files.len();
                        if len > 0 {
                            let cur = state.file_state.selected().unwrap_or(0);
                            state.file_state.select(Some(cur.saturating_sub(1)));
                        }
                    }
                    _ => state.prev_focus(),
                },
                (KeyCode::Down, _) => match state.focus {
                    Focus::Dependencies => {
                        let len = state.ctx.available_deps.len();
                        if len > 0 {
                            let cur = state.dep_state.selected().unwrap_or(0);
                            let next = (cur + 1).min(len - 1);
                            state.dep_state.select(Some(next));
                        }
                    }
                    Focus::Files => {
                        let len = state.ctx.available_files.len();
                        if len > 0 {
                            let cur = state.file_state.selected().unwrap_or(0);
                            let next = (cur + 1).min(len - 1);
                            state.file_state.select(Some(next));
                        }
                    }
                    _ => state.next_focus(),
                },
                _ => {
                    // Route to active text field
                    match state.focus {
                        Focus::Description => {
                            state.desc_area.input(key);
                        }
                        Focus::Project => {
                            state.project_area.input(key);
                        }
                        Focus::Due => {
                            state.due_area.input(key);
                            state.validate_due();
                        }
                        Focus::Tags => {
                            state.tags_area.input(key);
                        }
                        _ => {}
                    }
                }
            }
        }

        if state.submitted {
            return Ok(Some(state.collect_result()));
        }
        if state.cancelled {
            return Ok(None);
        }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(f: &mut Frame, state: &mut FormState) {
    let area = f.area();
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" tk — Review Task "),
        area,
    );

    let inner = shrink(area, 1);

    // Split: fields on top, footer at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    render_fields(f, state, chunks[0]);
    render_footer(f, state, chunks[1]);
}

fn shrink(r: Rect, n: u16) -> Rect {
    Rect {
        x: r.x + n,
        y: r.y + n,
        width: r.width.saturating_sub(n * 2),
        height: r.height.saturating_sub(n * 2),
    }
}

fn render_fields(f: &mut Frame, state: &mut FormState, area: Rect) {
    // Heights: desc=3, proj=3, pri=3, due=3, tags=3, deps=5, files=5, buttons=3
    let heights = [3u16, 3, 3, 3, 3, 5, 5, 3];
    let _total: u16 = heights.iter().sum();
    if area.height < 4 {
        return;
    }

    let constraints: Vec<Constraint> = heights.iter().map(|&h| Constraint::Length(h)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // ── Description
    {
        let focused = state.focus == Focus::Description;
        let block = field_block("Description", focused);
        let inner = block.inner(rows[0]);
        f.render_widget(block, rows[0]);
        state.desc_area.set_block(Block::default());
        f.render_widget(&state.desc_area, inner);
    }

    // ── Project
    {
        let focused = state.focus == Focus::Project;
        let block = field_block("Project", focused);
        let inner = block.inner(rows[1]);
        f.render_widget(block, rows[1]);
        state.project_area.set_block(Block::default());
        f.render_widget(&state.project_area, inner);
    }

    // ── Priority
    {
        let focused = state.focus == Focus::Priority;
        let block = field_block("Priority  ←/→ to cycle", focused);
        let inner = block.inner(rows[2]);
        f.render_widget(block, rows[2]);
        let label = match &state.priority {
            None => Span::styled("None", Style::default().fg(Color::DarkGray)),
            Some(Priority::L) => Span::styled("L  (Low)", Style::default().fg(Color::Green)),
            Some(Priority::M) => Span::styled("M  (Medium)", Style::default().fg(Color::Yellow)),
            Some(Priority::H) => Span::styled("H  (High)", Style::default().fg(Color::Red)),
        };
        f.render_widget(Paragraph::new(Line::from(label)), inner);
    }

    // ── Due
    {
        let focused = state.focus == Focus::Due;
        let title = if state.due_error {
            "Due  ⚠ invalid date"
        } else {
            "Due  ←/→ presets, or type (2026-06-20, friday, +3d)"
        };
        let block = if state.due_error {
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Red))
        } else {
            field_block(title, focused)
        };
        let inner = block.inner(rows[3]);
        f.render_widget(block, rows[3]);
        state.due_area.set_block(Block::default());
        f.render_widget(&state.due_area, inner);
    }

    // ── Tags
    {
        let focused = state.focus == Focus::Tags;
        let block = field_block("Tags  (comma-separated)", focused);
        let inner = block.inner(rows[4]);
        f.render_widget(block, rows[4]);
        state.tags_area.set_block(Block::default());
        f.render_widget(&state.tags_area, inner);
    }

    // ── Dependencies
    {
        let focused = state.focus == Focus::Dependencies;
        let block = field_block("Dependencies  (space to toggle)", focused);
        let inner = block.inner(rows[5]);
        f.render_widget(block, rows[5]);
        if state.ctx.available_deps.is_empty() {
            f.render_widget(
                Paragraph::new("No existing tasks").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
        } else {
            let items: Vec<ListItem> = state
                .ctx
                .available_deps
                .iter()
                .enumerate()
                .map(|(i, (id, desc))| {
                    let check = if state.selected_deps[i] { "☑" } else { "☐" };
                    let suggested = state.ctx.suggested_dep_indices.contains(&i);
                    let style = if state.selected_deps[i] {
                        Style::default().fg(Color::Green)
                    } else if suggested {
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("{check} {id}  {desc}")).style(style)
                })
                .collect();
            let list = List::new(items).highlight_style(
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD),
            );
            f.render_stateful_widget(list, inner, &mut state.dep_state);
        }
    }

    // ── Files
    {
        let focused = state.focus == Focus::Files;
        let block = field_block("Relevant Files  (space to toggle)", focused);
        let inner = block.inner(rows[6]);
        f.render_widget(block, rows[6]);
        if state.ctx.available_files.is_empty() {
            f.render_widget(
                Paragraph::new("No files found").style(Style::default().fg(Color::DarkGray)),
                inner,
            );
        } else {
            let items: Vec<ListItem> = state
                .ctx
                .available_files
                .iter()
                .enumerate()
                .map(|(i, path)| {
                    let check = if state.selected_files[i] { "☑" } else { "☐" };
                    let suggested = state.ctx.suggested_file_indices.contains(&i);
                    let style = if state.selected_files[i] {
                        Style::default().fg(Color::Green)
                    } else if suggested {
                        Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!("{check} {path}")).style(style)
                })
                .collect();
            let list = List::new(items).highlight_style(
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD),
            );
            f.render_stateful_widget(list, inner, &mut state.file_state);
        }
    }

    // ── Submit / Cancel
    {
        let row = rows[7];
        let halves = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(row);

        let submit_style = if state.focus == Focus::Submit {
            Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)
        } else if state.can_submit() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        f.render_widget(
            Paragraph::new(" ✔  Save  (Ctrl+S)")
                .style(submit_style)
                .block(Block::default().borders(Borders::ALL)),
            halves[0],
        );

        let cancel_style = if state.focus == Focus::Cancel {
            Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        f.render_widget(
            Paragraph::new(" ✖  Cancel  (Esc)")
                .style(cancel_style)
                .block(Block::default().borders(Borders::ALL)),
            halves[1],
        );
    }
}

fn render_footer(f: &mut Frame, _state: &FormState, area: Rect) {
    let text = " Tab/Shift+Tab: move  •  ←/→: cycle priority  •  Space: toggle  •  Ctrl+S: save  •  Esc: cancel ";
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn field_block(title: &str, focused: bool) -> Block<'_> {
    if focused {
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {title} "))
            .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {title} "))
            .border_style(Style::default().fg(Color::DarkGray))
    }
}

