use tauri::{plugin::TauriPlugin, Runtime, Url, Webview};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("links")
        .on_navigation(|webview, url| {
            if !leaves_squadron(url) {
                return true;
            }
            open(webview, url.as_str());
            false
        })
        .build()
}

fn leaves_squadron(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && !matches!(url.host_str(), Some(host) if host == "localhost" || host == "127.0.0.1" || host.ends_with(".localhost"))
}

fn open<R: Runtime>(webview: &Webview<R>, url: &str) {
    #[cfg(target_os = "linux")]
    {
        let context = webview
            .window()
            .gtk_window()
            .ok()
            .and_then(|w| gtk::prelude::WidgetExt::display(&w).app_launch_context())
            .map(gtk::prelude::Cast::upcast)
            .unwrap_or_else(gtk::gio::AppLaunchContext::new);
        if let Err(e) = crate::launch(url, context) {
            log::error!("{url} did not open: {e}");
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = webview;
        if let Err(e) = tauri_plugin_opener::open_url(url, None::<&str>) {
            log::error!("{url} did not open: {e}");
        }
    }
}
