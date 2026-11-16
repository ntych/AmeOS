use crate::screen::{Action, Screen};
use crate::theme::{Theme, LOGO};
use crossterm::{
    cursor::MoveTo,
    event::KeyCode,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};

pub struct Menu {
    title: String,
    items: Vec<String>,
    selected: usize,
    status: String,
    theme: Theme,
    show_logo: bool,
}

impl Menu {
    pub fn new(title: &str, items: Vec<String>) -> Self {
        let show_logo = title == "AmeOS Loader";
        Self {
            title: title.to_owned(),
            items,
            selected: 0,
            status: "Ready".to_owned(),
            theme: Theme::ameos(),
            show_logo,
        }
    }
}

impl Screen for Menu {
    fn draw(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

        let mut line = 0;

        if self.show_logo {
            execute!(stdout, SetForegroundColor(self.theme.border))?;
            for logo_line in LOGO.lines() {
                execute!(stdout, MoveTo(0, line), Print(logo_line))?;
                line += 1;
            }
            execute!(stdout, ResetColor)?;
            line += 1;
        }

        execute!(
            stdout,
            MoveTo(0, line),
            SetForegroundColor(self.theme.border),
            Print("╔"),
            Print("═".repeat(self.title.len() + 2)),
            Print("╗"),
            ResetColor
        )?;
        line += 1;

        execute!(
            stdout,
            MoveTo(0, line),
            SetForegroundColor(self.theme.border),
            Print("║ "),
            SetForegroundColor(self.theme.title),
            Print(&self.title),
            SetForegroundColor(self.theme.border),
            Print(" ║"),
            ResetColor
        )?;
        line += 1;

        execute!(
            stdout,
            MoveTo(0, line),
            SetForegroundColor(self.theme.border),
            Print("╚"),
            Print("═".repeat(self.title.len() + 2)),
            Print("╝"),
            ResetColor
        )?;
        line += 2;

        execute!(
            stdout,
            MoveTo(0, line),
            SetForegroundColor(Color::DarkGrey),
            Print("Use ↑/↓ to navigate, Enter to select, Esc to go back"),
            ResetColor
        )?;
        line += 2;

        for (index, item) in self.items.iter().enumerate() {
            if index == self.selected {
                execute!(
                    stdout,
                    MoveTo(0, line),
                    SetForegroundColor(self.theme.selected),
                    Print("► "),
                    Print(item),
                    ResetColor
                )?;
            } else {
                execute!(
                    stdout,
                    MoveTo(0, line),
                    SetForegroundColor(self.theme.normal),
                    Print("  "),
                    Print(item),
                    ResetColor
                )?;
            }
            line += 1;
        }

        line += 1;
        execute!(
            stdout,
            MoveTo(0, line),
            SetForegroundColor(self.theme.status),
            Print(&self.status),
            ResetColor
        )?;

        stdout.flush()
    }

    fn handle_key(&mut self, code: KeyCode) -> Action {
        match code {
            KeyCode::Up => {
                if self.items.is_empty() {
                    return Action::Continue;
                }
                self.selected = self.selected.checked_sub(1).unwrap_or(self.items.len() - 1);
                self.status = "Ready".to_owned();
                Action::Continue
            }
            KeyCode::Down => {
                if self.items.is_empty() {
                    return Action::Continue;
                }
                self.selected = (self.selected + 1) % self.items.len();
                self.status = "Ready".to_owned();
                Action::Continue
            }
            KeyCode::Enter => Action::Select,
            KeyCode::Esc => Action::Back,
            _ => Action::Continue,
        }
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn selected_index(&self) -> Option<usize> {
        Some(self.selected)
    }

    fn set_status(&mut self, msg: &str) {
        self.status = msg.to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::Action;
    use crossterm::event::KeyCode;

    fn menu(items: &[&str]) -> Menu {
        Menu::new("Test", items.iter().map(|s| s.to_string()).collect())
    }

    fn is_continue(a: Action) -> bool {
        matches!(a, Action::Continue)
    }
    fn is_select(a: Action) -> bool {
        matches!(a, Action::Select)
    }
    fn is_back(a: Action) -> bool {
        matches!(a, Action::Back)
    }

    // ── Enter / Esc ───────────────────────────────────────────────────────────

    #[test]
    fn enter_produces_select() {
        let mut m = menu(&["A", "B"]);
        assert!(is_select(m.handle_key(KeyCode::Enter)));
    }

    #[test]
    fn esc_produces_back() {
        let mut m = menu(&["A", "B"]);
        assert!(is_back(m.handle_key(KeyCode::Esc)));
    }

    #[test]
    fn unrecognised_key_produces_continue() {
        let mut m = menu(&["A"]);
        assert!(is_continue(m.handle_key(KeyCode::Tab)));
        assert!(is_continue(m.handle_key(KeyCode::Char('x'))));
    }

    // ── Down navigation ───────────────────────────────────────────────────────

    #[test]
    fn down_advances_selection() {
        let mut m = menu(&["A", "B", "C"]);
        assert_eq!(m.selected_index(), Some(0));
        m.handle_key(KeyCode::Down);
        assert_eq!(m.selected_index(), Some(1));
        m.handle_key(KeyCode::Down);
        assert_eq!(m.selected_index(), Some(2));
    }

    #[test]
    fn down_wraps_at_end() {
        let mut m = menu(&["A", "B"]);
        m.handle_key(KeyCode::Down); // 0 → 1
        m.handle_key(KeyCode::Down); // 1 → 0 (wrap)
        assert_eq!(m.selected_index(), Some(0));
    }

    // ── Up navigation ─────────────────────────────────────────────────────────

    #[test]
    fn up_moves_selection_back() {
        let mut m = menu(&["A", "B", "C"]);
        m.handle_key(KeyCode::Down); // → 1
        m.handle_key(KeyCode::Up); // → 0
        assert_eq!(m.selected_index(), Some(0));
    }

    #[test]
    fn up_wraps_at_start() {
        let mut m = menu(&["A", "B", "C"]);
        // cursor at 0; Up should jump to last item
        m.handle_key(KeyCode::Up);
        assert_eq!(m.selected_index(), Some(2));
    }

    // ── Empty-list guard (no panic on underflow) ──────────────────────────────

    #[test]
    fn up_on_empty_menu_does_not_panic() {
        let mut m = menu(&[]);
        let action = m.handle_key(KeyCode::Up);
        assert!(is_continue(action));
        assert_eq!(m.selected_index(), Some(0));
    }

    #[test]
    fn down_on_empty_menu_does_not_panic() {
        let mut m = menu(&[]);
        let action = m.handle_key(KeyCode::Down);
        assert!(is_continue(action));
    }

    // ── Single-item menu ──────────────────────────────────────────────────────

    #[test]
    fn single_item_down_stays_at_zero() {
        let mut m = menu(&["Only"]);
        m.handle_key(KeyCode::Down);
        assert_eq!(m.selected_index(), Some(0));
    }

    #[test]
    fn single_item_up_stays_at_zero() {
        let mut m = menu(&["Only"]);
        m.handle_key(KeyCode::Up);
        assert_eq!(m.selected_index(), Some(0));
    }

    // ── Title / logo flag ─────────────────────────────────────────────────────

    #[test]
    fn title_is_stored_correctly() {
        let m = menu(&["A"]);
        assert_eq!(m.title(), "Test");
    }

    #[test]
    fn main_menu_sets_show_logo() {
        let m = Menu::new("AmeOS Loader", vec!["Boot".to_owned()]);
        assert!(m.show_logo);
    }

    #[test]
    fn non_main_menu_does_not_set_show_logo() {
        let m = menu(&["A"]);
        assert!(!m.show_logo);
    }
}
