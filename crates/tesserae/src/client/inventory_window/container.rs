//! The open Container's slots beside the Inventory, drawn from the Stacks the
//! Server last sent. Every move is an intent the Server answers; walking away
//! or the Voxel changing closes it from the Server's side.

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use tesserae_core::inventory::Stack;

use super::{Built, Control, InventoryWindow, Names, Panel, look, slot_button};
use crate::client::buttons::{self, Eyebrow};
use crate::client::cinematic::Kind;
use crate::client::drops::Looks;
use crate::client::edit::Catalogue;
use crate::client::menu_ui as ui;
use crate::client::theme;
use crate::server::container::{Closed, ContainerClosed, ContainerShown};
use crate::server::inventory::{InventoryIntent, SlotAddress};
use crate::session::SessionApp;

/// The Container the Player has open, as the Server last sent it.
#[derive(Resource, Default)]
pub(super) struct Open(pub(super) Option<ContainerShown>);

impl Open {
    pub(super) fn stack(&self, at: usize) -> Option<&Stack> {
        self.0.as_ref()?.slots.get(at)?.as_ref()
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<Open>()
        .add_session_message::<ContainerShown>()
        .add_session_message::<ContainerClosed>();
}

/// Opens the window on a Container's first Stacks and keeps them current;
/// the Server's close takes the pane away with a notice.
pub(super) fn receive(
    mut commands: Commands,
    (mut shown, mut closed): (
        MessageReader<ContainerShown>,
        MessageReader<ContainerClosed>,
    ),
    (mut open, mut window): (ResMut<Open>, ResMut<InventoryWindow>),
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    (looks, catalogue): (Res<Looks>, Res<Catalogue>),
) {
    for message in shown.read() {
        if !window.open {
            super::open(&mut window, &mut cursor);
        }
        open.0 = Some(message.clone());
    }
    for message in closed.read() {
        let Some(was) = open.0.take_if(|o| o.at == message.at) else {
            continue;
        };
        forget(&mut window);
        let names = Names {
            looks: &looks,
            catalogue: &catalogue,
        };
        let name = names.material_label(was.material);
        let text = match message.why {
            Closed::OutOfReach => format!("Out of reach of the {name}."),
            Closed::Changed => format!("The {name} changed."),
        };
        ui::notify(&mut commands, &text, ui::Tone::Alert);
    }
}

/// A Container's slot picked or dragged is gone with its pane.
fn forget(window: &mut InventoryWindow) {
    let container = |a: Option<SlotAddress>| matches!(a, Some(SlotAddress::Container(_)));
    if container(window.picked) {
        window.picked = None;
    }
    if container(window.dragging) {
        window.dragging = None;
    }
}

/// Closing the window closes the session.
pub(super) fn leave(
    window: Res<InventoryWindow>,
    mut open: ResMut<Open>,
    mut intents: MessageWriter<InventoryIntent>,
) {
    if !window.open && open.0.take().is_some() {
        intents.write(InventoryIntent::CloseContainer);
    }
}

pub(super) fn panel(row: &mut ChildSpawnerCommands, state: &Built, names: &Names) {
    let Some(open) = &state.container else {
        return;
    };
    let used = open.slots.iter().flatten().count();
    row.spawn((
        Panel,
        Node {
            width: Val::Px(super::GRID_PX + 2.0 * PAD_PX),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            padding: UiRect::all(Val::Px(PAD_PX)),
            border: UiRect::all(Val::Px(theme::FRAME)),
            border_radius: BorderRadius::all(Val::Px(theme::ITEM_RADIUS)),
            ..default()
        },
        BackgroundColor(theme::ITEM_SURFACE),
        BorderColor::all(theme::GROUT),
        super::item_shadow(),
    ))
    .with_children(|pane| {
        pane.spawn(theme::ring(theme::ITEM_RADIUS));
        pane.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|title| {
            buttons::eyebrow(title, "CONTAINER", Eyebrow::Inventory);
            title.spawn(look::label(
                names.material_label(open.material),
                theme::ITEM_TEXT_TITLE,
                theme::MEDIUM,
                theme::INK,
            ));
            title.spawn(look::label(
                format!(
                    "{used} / {} slots · stays here when you leave",
                    open.slots.len()
                ),
                theme::TEXT_EYEBROW,
                theme::REGULAR,
                theme::item_capacity(),
            ));
        });
        pane.spawn(Node {
            width: Val::Px(super::GRID_PX),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(super::SLOT_GAP_PX),
            column_gap: Val::Px(super::SLOT_GAP_PX),
            ..default()
        })
        .with_children(|grid| {
            for (at, stack) in open.slots.iter().enumerate() {
                let slot = SlotAddress::Container(at);
                slot_button(
                    grid,
                    slot,
                    stack.as_ref(),
                    names,
                    state.picked == Some(slot),
                );
            }
        });
        pane.spawn(Node {
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|actions| {
            look::button(
                actions,
                "Take all",
                Control::TakeAll,
                Kind::Ghost,
                look::SMALL,
            );
            look::button(
                actions,
                "Sort",
                Control::SortContainer,
                Kind::Ghost,
                look::SMALL,
            );
        });
    });
}

const PAD_PX: f32 = 18.0;

#[cfg(test)]
mod tests {
    use bevy::ui_widgets::Activate;
    use tesserae_core::inventory::Slot;

    use super::super::tests::{app, press, server};
    use super::*;
    use crate::content::ItemLook;

    const CHEST: MaterialId = 3;
    const AT: [i32; 3] = [5, 8, 5];

    use tesserae_core::MaterialId;

    /// A window with the Server's Chest of 4 slots open beside it, 2 rock in
    /// its slot 1.
    fn opened() -> App {
        let mut app = app();
        app.insert_resource(Looks(vec![ItemLook {
            name: "t:chest".into(),
            size_m: 0.25,
            color: [0; 4],
            material: Some(CHEST),
            max_stack: 64,
        }]));
        let rock = Some(Stack {
            item: "t:rock".into(),
            count: 2,
        });
        app.world_mut().write_message(ContainerShown {
            at: AT,
            material: CHEST,
            slots: vec![None, rock, None, None],
        });
        server(&mut app);
        server(&mut app);
        app
    }

    fn texts(app: &mut App) -> Vec<String> {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }

    fn find(app: &mut App, wanted: Control) -> Option<Entity> {
        app.world_mut()
            .query::<(Entity, &Control)>()
            .iter(app.world())
            .find_map(|(entity, c)| (*c == wanted).then_some(entity))
    }

    fn sent(app: &mut App, control: Control) -> Vec<InventoryIntent> {
        let entity = find(app, control).expect("on screen");
        app.world_mut().trigger(Activate { entity });
        let intents = app
            .world()
            .resource::<Messages<InventoryIntent>>()
            .iter_current_update_messages()
            .copied()
            .collect();
        app.world_mut()
            .resource_mut::<Messages<InventoryIntent>>()
            .clear();
        intents
    }

    fn slot(at: usize) -> Control {
        Control::Slot(SlotAddress::Container(at))
    }

    #[test]
    fn the_servers_chest_opens_its_slots_beside_the_inventory() {
        let mut app = opened();
        assert!(app.world().resource::<InventoryWindow>().open);
        let shown = texts(&mut app);
        assert!(shown.iter().any(|t| t == "Chest"), "{shown:?}");
        assert!(shown.iter().any(|t| t.starts_with("1 / 4 slots")));
        assert!(find(&mut app, slot(3)).is_some());
        assert!(find(&mut app, slot(4)).is_none());
    }

    #[test]
    fn moves_quick_transfer_take_all_and_sort_cross_to_the_chest_through_the_server() {
        let mut app = opened();
        let backpack = Control::Slot(Slot::Backpack(0).into());
        assert!(sent(&mut app, backpack).is_empty(), "picked");
        assert_eq!(
            sent(&mut app, slot(2)),
            [InventoryIntent::Move {
                from: Slot::Backpack(0).into(),
                to: SlotAddress::Container(2),
            }]
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::ShiftLeft);
        assert_eq!(
            sent(&mut app, slot(1)),
            [InventoryIntent::QuickTransfer(SlotAddress::Container(1))]
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ShiftLeft);
        assert_eq!(sent(&mut app, Control::TakeAll), [InventoryIntent::TakeAll]);
        assert_eq!(
            sent(&mut app, Control::SortContainer),
            [InventoryIntent::SortContainer]
        );
    }

    #[test]
    fn the_server_closing_it_takes_the_pane_away_with_a_notice() {
        for (why, notice) in [
            (Closed::OutOfReach, "Out of reach of the Chest."),
            (Closed::Changed, "The Chest changed."),
        ] {
            let mut app = opened();
            app.world_mut()
                .write_message(ContainerClosed { at: AT, why });
            server(&mut app);
            server(&mut app);
            assert!(find(&mut app, slot(0)).is_none(), "{why:?}");
            assert!(texts(&mut app).iter().any(|t| t == notice), "{why:?}");
            assert!(app.world().resource::<InventoryWindow>().open);
        }
    }

    #[test]
    fn closing_the_window_closes_the_session() {
        let mut app = opened();
        press(&mut app, KeyCode::Escape);
        let messages = app.world().resource::<Messages<InventoryIntent>>();
        let intents: Vec<_> = messages.get_cursor().read(messages).copied().collect();
        assert_eq!(intents, [InventoryIntent::CloseContainer]);
        assert!(!app.world().resource::<InventoryWindow>().open);
        assert!(app.world().resource::<Open>().0.is_none());
        server(&mut app);
        assert!(find(&mut app, slot(0)).is_none());
    }
}
