use std::sync::atomic::{AtomicBool, Ordering};

use super::*;

/// Held until the test lets it go: a read of the Game that takes as long as the test needs.
static HELD: AtomicBool = AtomicBool::new(true);

fn held_preview(installed: &Installed, ids: &[String]) -> Result<BiomePreview, String> {
    let start = std::time::Instant::now();
    while HELD.load(Ordering::SeqCst) && start.elapsed().as_secs() < 30 {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    fake_preview(installed, ids)
}

fn release(app: &mut App) {
    HELD.store(false, Ordering::SeqCst);
    let start = std::time::Instant::now();
    while app.world().resource::<reads::Reads>().busy() {
        assert!(start.elapsed().as_secs() < 30, "the read never landed");
        std::thread::sleep(std::time::Duration::from_millis(1));
        app.update();
    }
    app.update();
}

fn reading(app: &mut App) -> bool {
    app.world_mut()
        .query::<&pages::Reading>()
        .iter(app.world())
        .next()
        .is_some()
}

// One test, so nothing else holds or lets go of the shared read meanwhile.
#[test]
fn next_shows_the_step_at_once_and_only_its_read_part_waits() {
    let worlds = Directory::new();
    let mut app = app(&worlds, Installed::fake(Some(("terra", "0.1.0")), &[]));
    app.insert_resource(Previewer(held_preview));
    app.world_mut().resource_mut::<reads::Reads>().wait = false;
    app.update();
    act(&mut app, Act::New);

    HELD.store(true, Ordering::SeqCst);
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::World));
    assert!(
        reading(&mut app),
        "the Climate scale shows it is being read"
    );
    assert_eq!(controls_of(&mut app, Field::TemperatureSize), 0);
    release(&mut app);
    assert!(!reading(&mut app));
    assert_eq!(controls_of(&mut app, Field::TemperatureSize), 1);

    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Mods));

    HELD.store(true, Ordering::SeqCst);
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Biomes));
    assert!(reading(&mut app), "the Biomes show they are being read");
    act(&mut app, Act::Next);
    assert_eq!(launcher(&app).screen, Screen::Create(Step::Review));
    act(&mut app, Act::Create);
    assert!(
        app.world().resource::<Switch>().0.is_none(),
        "no World is made before its Biomes are read"
    );
    release(&mut app);
    assert!(!reading(&mut app));
    assert!(shown(&mut app, "Meadow"), "Review lists the read Biomes");
}
