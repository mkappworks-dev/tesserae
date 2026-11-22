//! What the operating system shows for the app: the Dock icon and the menu bar.
//!
//! The release archive is a bare binary, not an `.app` bundle, so there is no
//! `Info.plist` or `.icns` to supply either; both are set through AppKit at
//! startup.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowCloseRequested};
use muda::{
    AboutMetadata, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, Code, Modifiers},
};
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::{NSData, NSProcessInfo, NSString};

const NAME: &str = "Tesserae";
const ICON: &[u8] = include_bytes!("../../../assets/icon.png");
const QUIT_ID: &str = "tesserae.quit";

pub(crate) fn plugin(app: &mut App) {
    // The menu is not `Send`, and winit installs its own default menu while the
    // event loop launches, so this runs on the main thread after that.
    app.add_systems(Startup, install)
        .add_systems(Update, forward_quit);
}

fn install(world: &mut World) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    // The app menu's title and its Hide and Quit items read the process name,
    // which for a bare binary is the lowercase executable name.
    NSProcessInfo::processInfo().setProcessName(&NSString::from_str(NAME));
    let data = NSData::with_bytes(ICON);
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        // SAFETY: the main thread, and the image outlives the call.
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
    }
    match build_menu() {
        Ok(menu) => {
            menu.init_for_nsapp();
            world.insert_non_send(menu);
        }
        Err(e) => warn!("the menu bar could not be built: {e}"),
    }
}

fn build_menu() -> muda::Result<Menu> {
    let about = AboutMetadata {
        name: Some(NAME.into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        icon: icon(),
        ..default()
    };
    // Not the predefined Quit: that terminates the process and skips the save
    // and the unsafe-quit confirmation the window's close button goes through.
    let quit = MenuItem::with_id(
        QUIT_ID,
        format!("Quit {NAME}"),
        true,
        Some(Accelerator::new(Some(Modifiers::SUPER), Code::KeyQ)),
    );
    let app = Submenu::with_items(
        NAME,
        true,
        &[
            &PredefinedMenuItem::about(None, Some(about)),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &quit,
        ],
    )?;
    let window = Submenu::with_items(
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(Some("Zoom")),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::fullscreen(None),
        ],
    )?;
    window.set_as_windows_menu_for_nsapp();
    Menu::with_items(&[&app, &window])
}

fn icon() -> Option<muda::Icon> {
    let image = Image::from_buffer(
        ICON,
        bevy::image::ImageType::Extension("png"),
        default(),
        true,
        default(),
        default(),
    )
    .ok()?;
    let (width, height) = (image.width(), image.height());
    muda::Icon::from_rgba(image.data?, width, height).ok()
}

fn forward_quit(
    window: Option<Single<Entity, With<PrimaryWindow>>>,
    mut requests: MessageWriter<WindowCloseRequested>,
) {
    while let Ok(event) = MenuEvent::receiver().try_recv() {
        if event.id.as_ref() == QUIT_ID
            && let Some(window) = &window
        {
            requests.write(WindowCloseRequested { window: **window });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_decodes_to_a_square() {
        assert!(icon().is_some());
    }
}
