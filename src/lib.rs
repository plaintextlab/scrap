mod app;
mod git;
mod markdown;
mod vault;

pub use app::App;

/// Entry point when the app is loaded as a native library by Android's NativeActivity.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android: winit::platform::android::activity::AndroidApp) {
    // Notes live in the app's external files dir (visible over USB / to file managers)
    // and fall back to private storage.
    let data = android.external_data_path().or_else(|| android.internal_data_path());
    let vault = data.as_ref().map(|d| d.join("vault"));
    let options = eframe::NativeOptions {
        android_app: Some(android),
        persistence_path: data,
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Scrap",
        options,
        Box::new(move |cc| Ok(Box::new(App::new(cc, vault)))),
    );
}
