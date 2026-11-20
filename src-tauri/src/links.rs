use tauri::{Manager, Runtime, Url, Webview, plugin::TauriPlugin};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("links")
        .on_navigation(|webview, url| {
            if crate::browser::tab(webview.label()) || !leaves_squadron(url) {
                return true;
            }
            open(webview, url.as_str());
            false
        })
        .build()
}

pub fn from_squadron(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => url.host_str() == Some("localhost"),
        "http" | "https" => {
            url.host_str() == Some("tauri.localhost")
                || (cfg!(debug_assertions) && url.host_str() == Some("localhost"))
        }
        _ => false,
    }
}

pub fn keep_home<R: Runtime>(webview: &Webview<R>, url: &Url) {
    if from_squadron(url) || crate::browser::tab(webview.label()) {
        return;
    }
    let home = if cfg!(debug_assertions) {
        webview.app_handle().config().build.dev_url.clone()
    } else if cfg!(windows) {
        Url::parse("http://tauri.localhost/").ok()
    } else {
        Url::parse("tauri://localhost/").ok()
    };
    log::warn!("{} left Squadron for {url}, sent back", webview.label());
    if let Some(home) = home {
        let _ = webview.navigate(home);
    }
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
            .unwrap_or_default();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_squadron_pages_are_squadron() {
        let page = |url: &str| from_squadron(&Url::parse(url).unwrap());
        assert!(page("tauri://localhost/"));
        assert!(page("http://tauri.localhost/index.html"));
        assert!(!page("artifact://localhost/a1/page/index.html"));
        assert!(!page("http://artifact.localhost/a1/page/index.html"));
        assert!(!page("extension://localhost/x/panel.html"));
        assert!(!page("https://example.com/"));
        assert!(!page("tauri://evil.example/"));
        assert!(!page("about:blank"));
    }
}
