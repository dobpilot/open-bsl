//! Общая дочерняя программа для проверок системного launcher и BSL-заданий.

use std::{path::PathBuf, time::Duration};

pub struct Scratch(pub PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

pub fn scratch() -> Scratch {
    let root = std::env::temp_dir().join(format!(
        "open-bsl-launch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&root).unwrap();
    Scratch(root)
}

pub fn native_child() {
    let args: Vec<String> = std::env::args().collect();
    let marker = std::fs::read("launch-test-marker").unwrap_or_default();
    if !args
        .windows(2)
        .any(|pair| pair == ["--exact", "native_child"])
        || (marker != b"open-bsl-child-17" && marker != b"open-bsl-child-wait")
    {
        return;
    }
    std::fs::write("received-arguments", args[1..].join("\n")).unwrap();
    if marker == b"open-bsl-child-wait" {
        std::fs::write("child-ready-pending", std::process::id().to_string()).unwrap();
        std::fs::rename("child-ready-pending", "child-ready").unwrap();
        // Родителю нужно проверить соседнее задание и отмену/shutdown,
        // прежде чем разрешить завершение реальной дочерней программы.
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        while !std::path::Path::new("child-continue").exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "нет команды продолжения"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        std::fs::write("child-completed", b"continued after waiter drop").unwrap();
    }
    std::process::exit(17);
}
