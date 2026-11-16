use std::io;
#[cfg(target_os = "freebsd")]
use std::process::Command;

#[derive(Debug, Clone)]
pub struct BootEnvironment {
    pub name: String,
    pub active: bool,
    #[allow(dead_code)]
    pub mountpoint: String,
    #[allow(dead_code)]
    pub creation: String,
}

/// Parse one line from `bectl list -H`. The output is tab-separated.
/// Columns: Name  Active  Mountpoint  Space  Created...
/// Returns None if the line has fewer than 5 tab-separated fields.
pub fn parse_bectl_line(line: &str) -> Option<BootEnvironment> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() >= 5 {
        Some(BootEnvironment {
            name: parts[0].to_owned(),
            active: parts[1].contains('N') || parts[1].contains('R'),
            mountpoint: parts[2].to_owned(),
            creation: parts[4..].join(" "),
        })
    } else {
        None
    }
}

pub fn list_boot_environments() -> io::Result<Vec<BootEnvironment>> {
    #[cfg(target_os = "freebsd")]
    {
        let output = Command::new("/usr/sbin/bectl")
            .arg("list")
            .arg("-H")
            .output()?;

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to list boot environments",
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut environments = Vec::new();

        for line in stdout.lines() {
            if let Some(be) = parse_bectl_line(line) {
                environments.push(be);
            }
        }

        Ok(environments)
    }

    #[cfg(not(target_os = "freebsd"))]
    {
        Ok(vec![
            BootEnvironment {
                name: "default".to_owned(),
                active: true,
                mountpoint: "/".to_owned(),
                creation: "2026-11-13".to_owned(),
            },
            BootEnvironment {
                name: "backup-20261110".to_owned(),
                active: false,
                mountpoint: "-".to_owned(),
                creation: "2026-11-10".to_owned(),
            },
            BootEnvironment {
                name: "test".to_owned(),
                active: false,
                mountpoint: "-".to_owned(),
                creation: "2026-11-05".to_owned(),
            },
        ])
    }
}

pub fn activate_boot_environment(name: &str) -> io::Result<()> {
    #[cfg(target_os = "freebsd")]
    {
        let status = Command::new("/usr/sbin/bectl")
            .arg("activate")
            .arg(name)
            .status()?;

        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to activate boot environment: {}", name),
            ));
        }

        Ok(())
    }

    #[cfg(not(target_os = "freebsd"))]
    {
        println!("Not on FreeBSD: would execute:");
        println!("  /usr/sbin/bectl activate {}", name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_bectl_line ──────────────────────────────────────────────────────

    #[test]
    fn parses_active_be() {
        // NR = currently mounted and nominated for next boot
        let line = "default\tNR\t/\t1.20G\t2026-11-13 15:42";
        let be = parse_bectl_line(line).expect("should parse");
        assert_eq!(be.name, "default");
        assert!(be.active, "NR flags must be treated as active");
        assert_eq!(be.mountpoint, "/");
        assert_eq!(be.creation, "2026-11-13 15:42");
    }

    #[test]
    fn parses_inactive_be() {
        let line = "backup-20261110\t-\t-\t980M\t2026-11-10 11:00";
        let be = parse_bectl_line(line).expect("should parse");
        assert_eq!(be.name, "backup-20261110");
        assert!(!be.active, "dash flag must be treated as inactive");
        assert_eq!(be.creation, "2026-11-10 11:00");
    }

    #[test]
    fn active_flag_n_only() {
        // N without R — activated for next boot but not mounted
        let line = "new-be\tN\t-\t500M\t2026-11-15 09:00";
        let be = parse_bectl_line(line).expect("should parse");
        assert!(be.active);
    }

    #[test]
    fn active_flag_r_only() {
        // R without N — currently mounted but not activated for next boot
        let line = "current-be\tR\t/\t1.5G\t2026-11-01 08:00";
        let be = parse_bectl_line(line).expect("should parse");
        assert!(be.active);
    }

    #[test]
    fn rejects_short_line() {
        // bectl -H uses tabs; lines with fewer than 5 tab-separated fields are ignored
        assert!(parse_bectl_line("name\tNR\t/\t1G").is_none()); // 4 tab-fields
        assert!(parse_bectl_line("a\tb\tc").is_none()); // 3 tab-fields
        assert!(parse_bectl_line("").is_none()); // empty → 1 field
    }

    #[test]
    fn creation_field_joins_date_and_time() {
        // The Created column can be split across two tokens (date + time).
        let line = "myenv\t-\t-\t200M\t2026-11-15\t09:30:00";
        let be = parse_bectl_line(line).expect("should parse");
        assert_eq!(be.creation, "2026-11-15 09:30:00");
    }
}
