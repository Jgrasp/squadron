use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::Manager;
use webkit2gtk::WebViewExt;

pub fn watch(app: tauri::AppHandle, dir: PathBuf) {
    std::thread::spawn(move || {
        let mut listening = None;
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let Some(main) = app.get_webview_window("main") else {
                continue;
            };
            if let Ok(js) = std::fs::read_to_string(dir.join("run.js")) {
                let _ = std::fs::remove_file(dir.join("run.js"));
                let _ = main.eval(&js);
                listening = Some(Instant::now() + Duration::from_secs(120));
            }
            if listening.is_some_and(|until| Instant::now() < until) {
                let answers = dir.join("answers");
                let _ = main.eval_with_callback(
                    "JSON.stringify((window.__drive ||= []).splice(0))",
                    move |out| {
                        if let Ok(text) = serde_json::from_str::<String>(&out)
                            && text != "[]"
                            && let Ok(mut file) = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&answers)
                        {
                            let _ = writeln!(file, "{text}");
                        }
                    },
                );
            }
            if std::fs::remove_file(dir.join("shot")).is_ok() {
                for (label, webview) in app.webview_windows() {
                    let path = dir.join(format!("{label}.png"));
                    let _ = webview.with_webview(move |p| {
                        p.inner().snapshot(
                            webkit2gtk::SnapshotRegion::Visible,
                            webkit2gtk::SnapshotOptions::NONE,
                            None::<&gtk::gio::Cancellable>,
                            move |r| {
                                if let Err(e) = save(r, &path) {
                                    log::warn!("dev shot {}: {e}", path.display());
                                }
                            },
                        );
                    });
                }
            }
        }
    });
}

fn save(r: Result<gtk::cairo::Surface, gtk::glib::Error>, path: &Path) -> Result<(), String> {
    let surface = gtk::cairo::ImageSurface::try_from(r.map_err(|e| e.to_string())?)
        .map_err(|_| "no image".to_string())?;
    let (w, h, stride) = (surface.width(), surface.height(), surface.stride() as usize);
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    surface
        .with_data(|d| {
            for y in 0..h as usize {
                for x in 0..w as usize {
                    // cairo ARGB32 is native-endian: B, G, R, A in memory on little-endian machines.
                    let i = y * stride + x * 4;
                    rgb.extend([d[i + 2], d[i + 1], d[i]]);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    let part = path.with_extension("part");
    gtk::gdk_pixbuf::Pixbuf::from_bytes(
        &gtk::glib::Bytes::from_owned(rgb),
        gtk::gdk_pixbuf::Colorspace::Rgb,
        false,
        8,
        w,
        h,
        w * 3,
    )
    .savev(&part, "png", &[])
    .map_err(|e| e.to_string())?;
    std::fs::rename(&part, path).map_err(|e| e.to_string())
}
