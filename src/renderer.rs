use image::DynamicImage;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

#[derive(Debug, Clone)]
pub enum RenderState {
    Idle,
    Rendering,
    DoneDecoded(DynamicImage),
    Error(String),
}

pub fn spawn_dot_render(
    src: PathBuf,
    command: String,
    args: Vec<String>,
    tx: mpsc::Sender<RenderState>,
) {
    thread::spawn(move || {
        let _ = tx.send(RenderState::Rendering);

        let output = std::process::Command::new(&command)
            .args(&args)
            .arg(&src)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                match image::load_from_memory(&out.stdout) {
                    Ok(dyn_img) => {
                        let _ = tx.send(RenderState::DoneDecoded(dyn_img));
                    }
                    Err(e) => {
                        let _ = tx.send(RenderState::Error(format!("Failed to decode PNG: {}", e)));
                    }
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let _ = tx.send(RenderState::Error(stderr));
            }
            Err(e) => {
                let _ = tx.send(RenderState::Error(format!("Failed to run `{}`: {}", command, e)));
            }
        }
    });
}
