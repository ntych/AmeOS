mod backend;
mod kernel;
mod menu;
mod screen;
mod snapshot;
mod theme;
mod zfs;

use backend::BootBackend;
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyEvent, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use menu::Menu;
use screen::{Action, Screen};
use std::io;

#[cfg(target_os = "freebsd")]
use backend::FreeBSDBackend;
#[cfg(not(target_os = "freebsd"))]
use backend::StubBackend;

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
    }
}

fn run_screen(screen: &mut dyn Screen) -> io::Result<Action> {
    screen.draw()?;
    loop {
        let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event::read()?
        else {
            continue;
        };
        match screen.handle_key(code) {
            Action::Continue => screen.draw()?,
            action => return Ok(action),
        }
    }
}

fn main() -> io::Result<()> {
    let terminal_guard = TerminalGuard::enter()?;

    #[cfg(target_os = "freebsd")]
    let mut backend = FreeBSDBackend::new();
    #[cfg(not(target_os = "freebsd"))]
    let backend = StubBackend;

    let main_menu = Menu::new(
        "AmeOS Loader",
        vec![
            "Boot AmeOS".to_owned(),
            "Select kernel".to_owned(),
            "Boot profiles".to_owned(),
            "Boot environments (ZFS)".to_owned(),
            "Boot recovery environment".to_owned(),
            "Hardware diagnostics".to_owned(),
            "Snapshot info".to_owned(),
            "Power off".to_owned(),
        ],
    );

    let mut stack: Vec<Box<dyn Screen>> = vec![Box::new(main_menu)];

    'main: while let Some(mut current) = stack.pop() {
        // TODO(tech-debt): dispatch is done by comparing title strings; a
        // Screen::on_select() method per screen would be cleaner and would
        // prevent silent routing breaks from title typos.
        let action = run_screen(&mut *current)?;
        let title = current.title();
        let selected = current.selected_index().unwrap_or(0);

        match action {
            Action::Continue => unreachable!(),
            // Esc: go back up the stack, or exit from main menu
            Action::Back => {
                if stack.is_empty() {
                    break 'main;
                }
                // Discard current screen, resume the parent
            }
            // Enter: act on the selected item
            Action::Select => {
                if title == "AmeOS Loader" {
                    match selected {
                        0 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.boot_default()?;
                            return Ok(());
                        }
                        1 => {
                            stack.push(current);
                            let kernels = kernel::list_available_kernels();
                            let current_kernel = kernel::get_current_kernel().unwrap_or_default();
                            let mut items: Vec<String> = kernels
                                .iter()
                                .map(|k| {
                                    let status = if !k.available {
                                        " (unavailable)"
                                    } else if k.path == current_kernel {
                                        " (current)"
                                    } else {
                                        ""
                                    };
                                    format!("{} — {}{}", k.name, k.description, status)
                                })
                                .collect();
                            items.push("Back".to_owned());
                            stack.push(Box::new(Menu::new("Select Kernel", items)));
                        }
                        2 => {
                            stack.push(current);
                            stack.push(Box::new(Menu::new(
                                "Boot Profiles",
                                vec![
                                    "Default".to_owned(),
                                    "Single user mode".to_owned(),
                                    "Safe mode".to_owned(),
                                    "Back".to_owned(),
                                ],
                            )));
                        }
                        3 => {
                            stack.push(current);
                            let environments =
                                zfs::list_boot_environments().unwrap_or_else(|_| vec![]);
                            let mut items: Vec<String> = environments
                                .iter()
                                .map(|be| {
                                    if be.active {
                                        format!("{} (active)", be.name)
                                    } else {
                                        be.name.clone()
                                    }
                                })
                                .collect();
                            items.push("Back".to_owned());
                            stack.push(Box::new(Menu::new("Boot Environments (ZFS)", items)));
                        }
                        4 => {
                            stack.push(current);
                            stack.push(Box::new(Menu::new(
                                "Recovery Environment",
                                vec![
                                    "Mount filesystems".to_owned(),
                                    "Network configuration".to_owned(),
                                    "Shell".to_owned(),
                                    "Back".to_owned(),
                                ],
                            )));
                        }
                        5 => {
                            stack.push(current);
                            stack.push(Box::new(Menu::new(
                                "Hardware Diagnostics",
                                vec![
                                    "Memory test".to_owned(),
                                    "Disk test".to_owned(),
                                    "CPU information".to_owned(),
                                    "Back".to_owned(),
                                ],
                            )));
                        }
                        6 => {
                            drop(terminal_guard); // restores terminal before backend action
                            match snapshot::SnapshotInfo::current() {
                                Ok(info) => {
                                    println!("{}", info.display());
                                    println!("Press Enter to continue...");
                                    let mut input = String::new();
                                    io::stdin().read_line(&mut input)?;
                                }
                                Err(e) => {
                                    eprintln!("Failed to get snapshot info: {}", e);
                                }
                            }
                            return Ok(());
                        }
                        7 => {
                            drop(terminal_guard); // restores terminal before backend action
                            if let Err(e) = backend.power_off() {
                                eprintln!("Power off failed: {}", e);
                            }
                            return Ok(());
                        }
                        _ => {
                            stack.push(current);
                        }
                    }
                } else if title == "Select Kernel" {
                    let kernels = kernel::list_available_kernels();
                    if selected < kernels.len() {
                        let k = &kernels[selected];
                        if k.available {
                            #[cfg(target_os = "freebsd")]
                            backend.set_kernel(k.path.clone());
                            #[cfg(not(target_os = "freebsd"))]
                            let _ = k;
                            // Surface the new selection on the parent menu's status line.
                            if let Some(parent) = stack.last_mut() {
                                parent.set_status(&format!("Kernel set: {}", k.name));
                            }
                        } else if let Some(parent) = stack.last_mut() {
                            parent.set_status(&format!("Kernel unavailable: {}", k.name));
                        }
                    }
                    // Always return to parent regardless of which item was selected
                } else if title == "Boot Profiles" {
                    match selected {
                        0 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.boot_default()?;
                            return Ok(());
                        }
                        1 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.boot_single_user()?;
                            return Ok(());
                        }
                        2 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.boot_safe_mode()?;
                            return Ok(());
                        }
                        _ => {} // "Back" — fall through to pop the stack
                    }
                } else if title == "Recovery Environment" {
                    match selected {
                        0 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.mount_filesystems()?;
                            return Ok(());
                        }
                        1 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.configure_network()?;
                            return Ok(());
                        }
                        2 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.spawn_shell()?;
                            return Ok(());
                        }
                        _ => {} // "Back"
                    }
                } else if title == "Hardware Diagnostics" {
                    match selected {
                        0 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.run_memory_test()?;
                            return Ok(());
                        }
                        1 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.run_disk_test()?;
                            return Ok(());
                        }
                        2 => {
                            drop(terminal_guard); // restores terminal before backend action
                            backend.show_cpu_info()?;
                            return Ok(());
                        }
                        _ => {} // "Back"
                    }
                } else if title == "Boot Environments (ZFS)" {
                    let environments = zfs::list_boot_environments().unwrap_or_else(|_| vec![]);
                    if selected < environments.len() {
                        let be = &environments[selected];
                        drop(terminal_guard); // restores terminal before backend action
                        if let Err(e) = zfs::activate_boot_environment(&be.name) {
                            eprintln!("Failed to activate boot environment: {}", e);
                        } else {
                            println!("Boot environment '{}' activated.", be.name);
                            println!("It will be used on next boot.");
                        }
                        return Ok(());
                    }
                    // "Back" — fall through to pop the stack
                }
            }
        }
    }

    Ok(())
}
