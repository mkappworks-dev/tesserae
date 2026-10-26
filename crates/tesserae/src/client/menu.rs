//! The picker: an engine menu that puts anything the content catalogue lists
//! into the selected hotbar slot. Tab opens and closes it, and Escape closes it
//! too. It lists the catalogue in the engine's own order; the Game has no say.
//!
//! While it is open the cursor is free, so mouse look is off and a click edits
//! nothing: both already wait on a grabbed cursor.

use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
use bevy::picking::events::{PointerOver, PointerOut, PointerPress, PointerDragStart, PointerDragEnd, PointerDragDrop};
use bevy::picking::pointer::PointerButton;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use tesserae_core::{MaterialId, ShapeId};

use super::edit::{Catalogue, Holding, Hotbar};
use crate::content::{ContentCatalogue, MaterialPalette};
use crate::render::chunk_material::{ChunkMaterialExtension, FLAME_SLOT};

#[derive(Resource, Default)]
pub(super) struct Menu {
    pub(super) open: bool,
    /// The solid whose Shapes are listed. Unset, it is the held solid's
    /// Material, or else the first solid.
    material: Option<MaterialId>,
}

impl Menu {
    #[cfg(feature = "dev")]
    pub(super) fn opened() -> Self {
        Menu {
            open: true,
            material: None,
        }
    }

    fn material(&self, hotbar: &Hotbar, catalogue: &ContentCatalogue) -> Option<MaterialId> {
        self.material
            .or(match hotbar.held() {
                Holding::Solid { material, .. } => Some(material),
                _ => None,
            })
            .or_else(|| catalogue.materials.iter().find(|m| m.solid).map(|m| m.id))
    }
}

pub(super) fn closed(menu: Res<Menu>) -> bool {
    !menu.open
}

/// What choosing an entry does.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(super) enum Entry {
    /// Lists this solid's Shapes.
    Material(MaterialId),
    Solid {
        material: MaterialId,
        shape: ShapeId,
    },
    Fluid(MaterialId),
    Fire,
    /// Selects this hotbar slot.
    Slot(usize),
}

impl Entry {
    pub(super) fn holding(self) -> Option<Holding> {
        match self {
            Entry::Solid { material, shape } => Some(Holding::solid(material, shape)),
            Entry::Fluid(material) => Some(Holding::Fluid(material)),
            Entry::Fire => Some(Holding::Fire),
            Entry::Material(material) => Some(Holding::solid(material, tesserae_core::SHAPE_FULL_CUBE)),
            Entry::Slot(_) => None,
        }
    }
}

/// The placeholder colours the World is drawn in: the built-in defaults with
/// the Server's appearance rows over them, as `chunk_view` applies them.
#[derive(Resource)]
pub(super) struct Swatches([Vec4; FLAME_SLOT + 1]);

impl Default for Swatches {
    fn default() -> Self {
        Swatches(ChunkMaterialExtension::default().palette)
    }
}

impl Swatches {
    /// Opaque, so a glass or water swatch reads at a glance.
    fn colour(&self, slot: usize) -> Color {
        let rgba = self.0.get(slot).copied().unwrap_or(Vec4::ONE);
        Color::linear_rgb(rgba.x, rgba.y, rgba.z)
    }

    pub(super) fn rgba(&self, holding: Holding) -> Vec4 {
        let slot=match holding {Holding::Solid {material,..}|Holding::Fluid(material)=>material as usize,Holding::Fire=>FLAME_SLOT,Holding::Empty=>return Vec4::ZERO};
        self.0.get(slot).copied().unwrap_or(Vec4::ONE)
    }

    pub(super) fn of(&self, holding: Holding) -> Color {
        match holding {
            Holding::Solid { material, .. } | Holding::Fluid(material) => {
                self.colour(material as usize)
            }
            Holding::Fire => self.colour(FLAME_SLOT),
            Holding::Empty => Color::NONE,
        }
    }
}

#[derive(Component)]
struct MenuRoot;
#[derive(Component)]
struct ShapePanel;
#[derive(Component)]
pub(super) struct PlayBarRoot;
#[derive(Component)]
struct HeldName;

#[derive(Resource, Default)]
pub(super) struct PointerState {
    pub(super) hovered: Option<Entry>,
    drag: Option<Entry>,
    suppress_click: bool,
}


pub(super) fn plugin(app: &mut App) {
    app.init_resource::<Menu>()
        .init_resource::<Swatches>()
        .init_resource::<PointerState>()
        .add_observer(choose)
        .add_observer(hover)
        .add_observer(unhover)
        .add_observer(press)
        .add_observer(drag_start)
        .add_observer(drag_end)
        .add_observer(drop_on_slot)
        .add_systems(
            Update,
            (
                // Before the edit path, so it sees the grab a close makes and
                // the first click after one edits.
                toggle.before(super::edit::send_intents),
                receive_palette,
                rebuild.after(super::edit::send_intents),
                rebuild_shapes,
                rebuild_bar,
            )
                .chain(),
        );
}

/// Tab opens and closes; closing grabs the cursor for building. Escape closes
/// and leaves the cursor free, as it does outside the menu, so the next click
/// grabs it and edits nothing.
fn toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<Menu>,
    cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let tab = keys.just_pressed(KeyCode::Tab);
    let escape = keys.just_pressed(KeyCode::Escape) && menu.open;
    if !tab && !escape {
        return;
    }
    let open = tab && !menu.open;
    let grab = tab && menu.open;
    *menu = Menu {
        open,
        material: None,
    };
    let mut cursor = cursor.into_inner();
    cursor.grab_mode = if grab {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    cursor.visible = !grab;
}

fn receive_palette(mut messages: MessageReader<MaterialPalette>, mut swatches: ResMut<Swatches>) {
    for message in messages.read() {
        let mut extension = ChunkMaterialExtension {
            palette: swatches.0,
            ..default()
        };
        extension.apply_palette(message);
        swatches.0 = extension.palette;
    }
}

/// Picking runs before `Update`, so a choice lands before this frame's edit
/// path reads the hotbar.
fn choose(
    activate: On<Activate>,
    entries: Query<&Entry>,
    mut menu: ResMut<Menu>,
    mut hotbar: ResMut<Hotbar>,
    pointer: Res<PointerState>,
    mut cursors: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if pointer.suppress_click { return; }
    let Ok(&entry) = entries.get(activate.entity) else {
        return;
    };
    if !menu.open {
        return;
    }
    match entry {
        Entry::Slot(slot) => hotbar.select(slot),
        _ => {
            hotbar.outside = entry.holding();
            menu.open = false;
            for mut cursor in &mut cursors { cursor.grab_mode = CursorGrabMode::Locked; cursor.visible = false; }
        }
    }
}

fn hover(event: On<PointerOver>, entries: Query<&Entry>, mut state: ResMut<PointerState>, mut menu: ResMut<Menu>) {
    if !menu.open { return; }
    if let Ok(&entry) = entries.get(event.entity) {
        state.hovered = Some(entry);
        if let Entry::Material(material) | Entry::Solid { material, .. } = entry { menu.material = Some(material); }
    }
}
fn unhover(event: On<PointerOut>, entries: Query<&Entry>, mut state: ResMut<PointerState>) {
    if entries.get(event.entity).is_ok_and(|entry| state.hovered == Some(*entry)) { state.hovered = None; }
}
fn press(event: On<PointerPress>, mut state: ResMut<PointerState>) {
    if event.button == PointerButton::Primary { state.suppress_click = false; }
}
fn drag_start(event: On<PointerDragStart>, entries: Query<&Entry>, menu: Res<Menu>, mut state: ResMut<PointerState>) {
    if menu.open && event.button == PointerButton::Primary && let Ok(&entry) = entries.get(event.entity) {
        state.drag = Some(entry); state.suppress_click = true;
    }
}
fn drag_end(_event: On<PointerDragEnd>, mut state: ResMut<PointerState>) { state.drag = None; }
fn drop_on_slot(event: On<PointerDragDrop>, entries: Query<&Entry>, menu: Res<Menu>, mut state: ResMut<PointerState>, mut hotbar: ResMut<Hotbar>) {
    if !menu.open || event.button != PointerButton::Primary { return; }
    let Ok(&Entry::Slot(target)) = entries.get(event.entity) else { return; };
    let source = state.drag.or_else(||entries.get(event.dropped).ok().copied());
    if let Some(source) = source {
        match source { Entry::Slot(from) => hotbar.slots.swap(from,target), item => if let Some(held) = item.holding() { hotbar.slots[target] = held; } }
        state.suppress_click = true;
    }
}

fn rebuild(
    mut commands: Commands,
    menu: Res<Menu>, hotbar: Res<Hotbar>, catalogue: Res<Catalogue>, swatches: Res<Swatches>,
    roots: Query<Entity, With<MenuRoot>>,
    mut state: ResMut<PointerState>,
    mut shown: Local<Option<Hotbar>>,
) {
    let wanted = menu.open.then(||hotbar.clone());
    if *shown == wanted && !catalogue.is_changed() && !swatches.is_changed() { return; }
    if state.drag.is_some() && menu.open { return; }
    *shown = wanted;
    state.hovered = None;
    for root in &roots { commands.entity(root).despawn(); }
    if shown.is_some() { spawn_menu(&mut commands, menu.material(&hotbar,&catalogue.0), &hotbar, &catalogue.0, &swatches); }
}

fn rebuild_shapes(mut commands: Commands, menu: Res<Menu>, hotbar: Res<Hotbar>, catalogue: Res<Catalogue>, swatches: Res<Swatches>, panels: Query<Entity, With<ShapePanel>>, mut shown: Local<Option<MaterialId>>) {
    let material = menu.material(&hotbar,&catalogue.0);
    if *shown == material && !catalogue.is_changed() { return; }
    *shown = material;
    for panel in &panels {
        commands.entity(panel).despawn_children().with_children(|list| {
            shape_tiles(list, material, &hotbar, &catalogue.0, &swatches);
        });
    }
}

fn rebuild_bar(mut commands:Commands, menu:Res<Menu>, hotbar:Res<Hotbar>, catalogue:Res<Catalogue>, swatches:Res<Swatches>, roots:Query<Entity,With<PlayBarRoot>>,mut shown:Local<Option<Hotbar>>) {
    let wanted=(!menu.open).then(||hotbar.clone());
    if *shown==wanted && !catalogue.is_changed() && !swatches.is_changed() {return;}
    *shown=wanted;
    for root in &roots {commands.entity(root).despawn();}
    if menu.open {return;}
    commands.spawn((PlayBarRoot,Node {
        position_type:PositionType::Absolute,bottom:Val::Px(10.0),width:Val::Percent(100.0),
        flex_direction:FlexDirection::Column,align_items:AlignItems::Center,row_gap:Val::Px(6.0),..default()
    })).with_children(|root| {
        root.spawn((HeldName,Text::new(hotbar.held().describe(&catalogue.0)),font(15.0),Pickable::IGNORE));
        root.spawn(Node {column_gap:Val::Px(5.0),align_items:AlignItems::FlexStart,..default()}).with_children(|bar| {
            for (slot,&holding) in hotbar.slots.iter().enumerate() {
                tile(bar,Entry::Slot(slot),&holding.describe(&catalogue.0),swatches.of(holding),hotbar.outside.is_none()&&hotbar.selected==slot,Some(slot+1),48.0);
            }
            if let Some(held)=hotbar.outside {
                bar.spawn(Node {margin:UiRect::left(Val::Px(12.0)),..default()}).with_children(|outside| {
                    let entry=match held {Holding::Solid {material,shape,..}=>Entry::Solid {material,shape},Holding::Fluid(material)=>Entry::Fluid(material),_=>Entry::Fire};
                    tile(outside,entry,&held.describe(&catalogue.0),swatches.of(held),true,None,48.0);
                });
            }
        });
    });
}

fn font(size: f32) -> TextFont {
    TextFont {
        font_size: bevy::text::FontSize::Px(size),
        ..default()
    }
}

fn spawn_menu(commands: &mut Commands, material: Option<MaterialId>, hotbar: &Hotbar, catalogue: &ContentCatalogue, swatches: &Swatches) {
    commands.spawn((MenuRoot, Node {
        position_type: PositionType::Absolute, left: Val::Percent(5.0), top: Val::Percent(5.0),
        width: Val::Percent(90.0), height: Val::Percent(90.0), overflow: Overflow::scroll_y(),
        flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), padding: UiRect::all(Val::Px(16.0)), ..default()
    }, BackgroundColor(Color::srgba(0.04,0.04,0.04,0.94)), GlobalZIndex(10))).with_children(|root| {
        root.spawn((Text::new("Click: hold item\nDrag onto a slot: assign\n1-9 over an item: assign\nTab: close\nEsc: close and release cursor"),font(14.0),Pickable::IGNORE));
        section(root,"Solids",|list| {
            for m in catalogue.materials.iter().filter(|m|m.solid && m.fluid.is_none()) {
                let entry = Entry::Material(m.id);
                tile(list,entry,&m.name,swatches.of(entry.holding().unwrap()), matches_item(hotbar.held(),entry),None,72.0);
            }
        });
        root.spawn((ShapePanel,Node { flex_direction:FlexDirection::Column,row_gap:Val::Px(4.0),..default() })).with_children(|panel|shape_tiles(panel,material,hotbar,catalogue,swatches));
        let mut fluids: Vec<_> = catalogue.materials.iter().filter_map(|m|Some((m.fluid?,m))).collect();
        fluids.sort_by_key(|(rank,_)|*rank);
        if !fluids.is_empty() { section(root,"Fluids",|list|for (_,m) in fluids {
            let entry=Entry::Fluid(m.id);tile(list,entry,&m.name,swatches.of(Holding::Fluid(m.id)),matches_item(hotbar.held(),entry),None,72.0);
        }); }
        if catalogue.anything_burns() { section(root,"Fire",|list|tile(list,Entry::Fire,"fire",swatches.of(Holding::Fire),hotbar.held()==Holding::Fire,None,72.0)); }
        section(root,"Slots",|list|for (slot,&holding) in hotbar.slots.iter().enumerate() {
            tile(list,Entry::Slot(slot),&holding.describe(catalogue),swatches.of(holding),hotbar.outside.is_none() && hotbar.selected==slot,Some(slot+1),72.0);
        });
    });
}
fn matches_item(held: Holding, entry: Entry) -> bool {
    match (held,entry.holding()) {
        (Holding::Solid { material:a,shape:x,.. },Some(Holding::Solid {material:b,shape:y,..})) => a==b && x==y,
        (a,Some(b)) => a==b, _=>false,
    }
}
fn shape_tiles(parent:&mut ChildSpawnerCommands, material:Option<MaterialId>, hotbar:&Hotbar, catalogue:&ContentCatalogue, swatches:&Swatches) {
    if let Some(material)=material {
        let name=catalogue.material(material).map_or("unknown",|m|m.name.as_str());
        section(parent,&format!("Shapes of {name}"),|list|for shape in &catalogue.shapes {
            let entry=Entry::Solid {material,shape:shape.id};
            tile(list,entry,&shape.name,swatches.of(entry.holding().unwrap()),matches_item(hotbar.held(),entry),None,72.0);
        });
    }
}
fn section(parent:&mut ChildSpawnerCommands,title:&str,entries:impl FnOnce(&mut ChildSpawnerCommands)) {
    parent.spawn(Node { flex_direction:FlexDirection::Column,row_gap:Val::Px(4.0),..default() }).with_children(|section| {
        section.spawn((Text::new(title),font(15.0),Pickable::IGNORE));
        section.spawn(Node { flex_wrap:FlexWrap::Wrap,column_gap:Val::Px(8.0),row_gap:Val::Px(8.0),..default() }).with_children(entries);
    });
}

pub(super) fn tile(parent:&mut ChildSpawnerCommands,entry:Entry,label:&str,colour:Color,highlighted:bool,number:Option<usize>,size:f32) {
    parent.spawn(Node {width:Val::Px(size),flex_direction:FlexDirection::Column,flex_shrink:0.0,row_gap:Val::Px(3.0),..default()}).with_children(|wrapper| {
        wrapper.spawn((entry,Button,Node {width:Val::Px(size),height:Val::Px(size),aspect_ratio:Some(1.0),flex_shrink:0.0,border:UiRect::all(Val::Px(2.0)),..default()},BackgroundColor(colour),BorderColor::all(if highlighted {Color::WHITE}else{Color::srgb(0.25,0.25,0.25)}))).with_children(|square| {
            if let Some(number)=number { square.spawn((Text::new(number.to_string()),font(12.0),Node {position_type:PositionType::Absolute,left:Val::Px(3.0),top:Val::Px(3.0),..default()},Pickable::IGNORE)); }
        });
        wrapper.spawn((Text::new(label),font(11.0),Node {max_width:Val::Px(size),..default()},Pickable::IGNORE));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::picking::{events::Pointer, pointer::{Location,PointerId}, backend::HitData};
    use bevy::camera::NormalizedRenderTarget;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::mouse::MouseWheel;

    fn pointer() -> Pointer { Pointer::new(PointerId::Mouse,Location { target:NormalizedRenderTarget::None {width:800,height:600},position:Vec2::ZERO }) }
    fn hit() -> HitData { HitData::new(Entity::PLACEHOLDER,0.0,None,None) }
    fn app() -> App {
        let mut app=App::new();
        app.insert_resource(Menu {open:true,material:None}).init_resource::<Hotbar>().init_resource::<PointerState>()
            .init_resource::<Catalogue>().init_resource::<ButtonInput<KeyCode>>().add_message::<MouseWheel>()
            .add_observer(choose).add_observer(hover).add_observer(unhover).add_observer(press).add_observer(drag_start).add_observer(drag_end).add_observer(drop_on_slot);
        app.world_mut().spawn((PrimaryWindow,CursorOptions::default()));app
    }
    #[test]
    fn picking_holds_outside_the_bar_and_closes_but_selecting_a_slot_keeps_the_menu_open() {
        let mut app=app(); let item=app.world_mut().spawn(Entry::Material(23)).id();
        let slots=app.world().resource::<Hotbar>().slots;
        app.world_mut().trigger(Activate {entity:item});
        assert_eq!(app.world().resource::<Hotbar>().outside,Some(Holding::solid(23,0)));
        assert_eq!(app.world().resource::<Hotbar>().slots,slots);
        assert!(!app.world().resource::<Menu>().open);
        app.world_mut().resource_mut::<Menu>().open=true;
        let slot=app.world_mut().spawn(Entry::Slot(4)).id();app.world_mut().trigger(Activate {entity:slot});
        assert!(app.world().resource::<Menu>().open);
        assert_eq!(app.world().resource::<Hotbar>().selected,4);
        assert_eq!(app.world().resource::<Hotbar>().outside,None);
    }
    #[test]
    fn dragging_assigns_and_swaps_without_quick_picking() {
        let mut app=app();let item=app.world_mut().spawn(Entry::Fluid(24)).id();
        let slot=app.world_mut().spawn(Entry::Slot(2)).id();
        app.world_mut().trigger(PointerDragStart {entity:item,pointer:pointer(),button:PointerButton::Primary,hit:hit()});
        app.world_mut().trigger(PointerDragDrop {entity:slot,dropped:item,pointer:pointer(),button:PointerButton::Primary,hit:hit()});
        app.world_mut().trigger(Activate {entity:item});
        assert!(app.world().resource::<Menu>().open);
        assert_eq!(app.world().resource::<Hotbar>().slots[2],Holding::Fluid(24));
        let other=app.world_mut().spawn(Entry::Slot(6)).id();
        app.world_mut().trigger(PointerDragDrop {entity:other,dropped:slot,pointer:pointer(),button:PointerButton::Primary,hit:hit()});
        // A second drag starts from the slot, independently of the prior item.
        app.world_mut().trigger(PointerDragStart {entity:slot,pointer:pointer(),button:PointerButton::Primary,hit:hit()});
        app.world_mut().trigger(PointerDragDrop {entity:other,dropped:slot,pointer:pointer(),button:PointerButton::Primary,hit:hit()});
        assert_eq!(app.world().resource::<Hotbar>().slots[6],Holding::Fluid(24));
        assert_eq!(app.world().resource::<Hotbar>().slots[2],Holding::Empty);
    }
    #[test]
    fn number_over_an_item_assigns_without_changing_selection() {
        let mut app=app();let item=app.world_mut().spawn(Entry::Material(23)).id();
        app.world_mut().trigger(PointerOver {entity:item,pointer:pointer(),hit:hit()});
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Digit5);
        app.world_mut().run_system_once(super::super::edit::select_held).unwrap();
        assert_eq!(app.world().resource::<Hotbar>().selected,0);
        assert_eq!(app.world().resource::<Hotbar>().slots[4],Holding::solid(23,0));
    }
    #[test]
    fn play_bar_tracks_the_hand_and_is_hidden_in_the_menu() {
        let mut app=app();
        app.init_resource::<Swatches>().add_systems(Update,rebuild_bar);
        app.update();
        assert_eq!(app.world_mut().query::<&PlayBarRoot>().iter(app.world()).count(),0);
        app.world_mut().resource_mut::<Menu>().open=false;
        app.world_mut().resource_mut::<Hotbar>().outside=Some(Holding::Fluid(24));
        app.update();
        assert_eq!(app.world_mut().query::<&PlayBarRoot>().iter(app.world()).count(),1);
        let squares=app.world_mut().query::<(&Entry,&Node)>().iter(app.world()).filter(|(_,node)|node.aspect_ratio==Some(1.0)).count();
        assert_eq!(squares,10);
        app.world_mut().resource_mut::<Hotbar>().select(2);app.update();
        let squares=app.world_mut().query::<(&Entry,&Node)>().iter(app.world()).filter(|(_,node)|node.aspect_ratio==Some(1.0)).count();
        assert_eq!(squares,9);
        app.world_mut().resource_mut::<Menu>().open=true;app.update();
        assert_eq!(app.world_mut().query::<&PlayBarRoot>().iter(app.world()).count(),0);
    }
}
