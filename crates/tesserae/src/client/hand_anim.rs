//! The hand's one animation state machine: idle, a swing for a Place, a steady
//! mining rhythm while a removal runs, bites for eating and a lower-and-raise
//! when the slot changes. It poses the first-person viewmodel (`viewmodel.rs`)
//! and, as [`Pose::arm_rad`], the avatar's right arm in third person.
//! Client-only, started by what the Server confirmed and never by an intent.

use super::{edit::Hotbar, profile::Preferences};
use crate::server::edit::{AppliedKind, EditApplied};
use crate::server::tool::UseOutcome;
use crate::server::vitals::EatIntent;
use crate::session::SessionApp;
use bevy::prelude::*;

const SWING_S: f32 = 0.2;
const SWING_FORWARD: f32 = 0.22;
const SWING_PITCH_RAD: f32 = 0.5;
const MINE_PERIOD_S: f32 = 0.35;
const BITE_S: f32 = 1.0;
const RISE_S: f32 = 0.25;
/// A slot change lowers the item and raises it again, the new one at the bottom.
const SWITCH_S: f32 = 0.24;
const SWITCH_DROP: f32 = 0.4;
const BITES: u32 = 3;
const BITE_EACH_S: f32 = 0.2;
const LOWER_S: f32 = BITE_S - RISE_S - BITES as f32 * BITE_EACH_S;
/// Where the food sits at the mouth, relative to the resting hand, in the
/// camera's frame.
const MOUTH: Vec3 = Vec3::new(-0.2, 0.27, 0.0);
/// How far the avatar's right arm swings forward at the far point of a Place or
/// a chop, and how far it is raised to bring food to the mouth.
pub(super) const ARM_SWING_RAD: f32 = 1.0;
const ARM_MOUTH_RAD: f32 = 2.0;
/// The viewmodel tips back at the elbow as the fist comes up, so the forearm
/// stays low in the view instead of crossing it.
const MOUTH_TILT_RAD: f32 = 0.7;
/// The food left in the hand after the last bite.
const FOOD_LEFT: f32 = 0.45;
/// How long after asking to eat the Server's answer may take.
const EAT_PATIENCE_S: f64 = 1.5;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Motion {
    Swing(f32),
    Bite(f32),
    Switch(f32),
}

/// How far the hand is from rest this frame.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Pose {
    pub(super) offset: Vec3,
    pub(super) pitch: f32,
    /// Of what the hand holds; below one only while it is being eaten.
    pub(super) scale: f32,
    /// How far the avatar's right arm is raised from its side, forward.
    pub(super) arm_rad: f32,
}

impl Pose {
    pub(super) const REST: Pose = Pose {
        offset: Vec3::ZERO,
        pitch: 0.0,
        scale: 1.0,
        arm_rad: 0.0,
    };
}

#[derive(Resource, Default, Debug)]
pub(super) struct HandAnim {
    motion: Option<Motion>,
    mining: bool,
    mine_s: f32,
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(2)
}

impl HandAnim {
    pub(super) fn swing(&mut self) {
        if !matches!(self.motion, Some(Motion::Bite(_))) {
            self.motion = Some(Motion::Swing(0.0));
        }
    }

    pub(super) fn bite(&mut self) {
        self.motion = Some(Motion::Bite(0.0));
    }

    /// The slot changed: lower the item and raise the next. A bite in progress
    /// is finished first, as a swing waits for it.
    pub(super) fn switch(&mut self) {
        if !matches!(self.motion, Some(Motion::Bite(_))) {
            self.motion = Some(Motion::Switch(0.0));
        }
    }

    /// Whether the item drawn may change now: not before a switch has lowered
    /// the old one out of sight.
    pub(super) fn swapped(&self) -> bool {
        !matches!(self.motion, Some(Motion::Switch(t)) if t < SWITCH_S / 2.0)
    }

    pub(super) fn set_mining(&mut self, mining: bool) {
        if mining && !self.mining {
            self.mine_s = 0.0;
        }
        self.mining = mining;
    }

    pub(super) fn playing(&self) -> bool {
        self.motion.is_some() || self.mining
    }

    pub(super) fn tick(&mut self, dt: f32) {
        if self.mining {
            self.mine_s = (self.mine_s + dt) % MINE_PERIOD_S;
        }
        self.motion = match self.motion {
            Some(Motion::Swing(t)) if t + dt < SWING_S => Some(Motion::Swing(t + dt)),
            Some(Motion::Bite(t)) if t + dt < BITE_S => Some(Motion::Bite(t + dt)),
            Some(Motion::Switch(t)) if t + dt < SWITCH_S => Some(Motion::Switch(t + dt)),
            _ => None,
        };
    }

    /// Reduced motion keeps the hand still: nothing here moves it.
    pub(super) fn pose(&self, reduced_motion: bool) -> Pose {
        if reduced_motion {
            return Pose::REST;
        }
        match self.motion {
            Some(Motion::Bite(t)) => bite_pose(t),
            Some(Motion::Swing(t)) => swing_pose(t / SWING_S),
            Some(Motion::Switch(t)) => switch_pose(t / SWITCH_S),
            None if self.mining => swing_pose(self.mine_s / MINE_PERIOD_S),
            None => Pose::REST,
        }
    }
}

/// Out and back over `0..1`: ease-out to the far point, then return.
fn swing_pose(phase: f32) -> Pose {
    let reach = if phase < 0.5 {
        ease_out(phase * 2.0)
    } else {
        1.0 - ease_out((phase - 0.5) * 2.0)
    };
    Pose {
        offset: Vec3::new(0.0, 0.0, -SWING_FORWARD * reach),
        pitch: -SWING_PITCH_RAD * reach,
        scale: 1.0,
        arm_rad: ARM_SWING_RAD * reach,
    }
}

/// Down and back up over `0..1`.
fn switch_pose(phase: f32) -> Pose {
    let low = (phase * std::f32::consts::PI).sin();
    Pose {
        offset: Vec3::new(0.0, -SWITCH_DROP * low, 0.0),
        ..Pose::REST
    }
}

fn bite_pose(t: f32) -> Pose {
    let bites_end = RISE_S + BITES as f32 * BITE_EACH_S;
    if t < RISE_S {
        let up = ease_out(t / RISE_S);
        return Pose {
            offset: MOUTH * up,
            pitch: MOUTH_TILT_RAD * up,
            scale: 1.0,
            arm_rad: ARM_MOUTH_RAD * up,
        };
    }
    if t < bites_end {
        let into = (t - RISE_S) / BITE_EACH_S;
        let bite = into.floor();
        let within = into - bite;
        // Each bite dips the food toward the mouth and takes a share of it.
        let dip = (within * std::f32::consts::PI).sin();
        let eaten = (bite + ease_out(within)) / BITES as f32;
        return Pose {
            offset: MOUTH + Vec3::new(-0.08, -0.04, 0.0) * dip,
            pitch: MOUTH_TILT_RAD + 0.25 * dip,
            scale: 1.0 - (1.0 - FOOD_LEFT) * eaten,
            arm_rad: ARM_MOUTH_RAD + 0.25 * dip,
        };
    }
    let down = ease_out((t - bites_end) / LOWER_S);
    Pose {
        offset: MOUTH * (1.0 - down),
        pitch: MOUTH_TILT_RAD * (1.0 - down),
        // The next bite of the stack is a whole Item again.
        scale: FOOD_LEFT + (1.0 - FOOD_LEFT) * down,
        arm_rad: ARM_MOUTH_RAD * (1.0 - down),
    }
}

/// An eat that has been asked and not yet answered, with the stack as it was.
#[derive(Default)]
struct AskedToEat {
    at_s: f64,
    before: Option<(String, u32)>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_session_state::<HandAnim>().add_session_systems(
        Update,
        (confirm, animate).chain().after(super::edit::follow_server),
    );
}

fn selected(hotbar: &Hotbar) -> Option<(String, u32)> {
    hotbar.stacks[hotbar.selected]
        .as_ref()
        .map(|stack| (stack.item.clone(), stack.count))
}

/// Starts motions from the Server's answers: a landed Place, a removal under
/// way, and a stack of food that shrank after an eat was asked.
fn confirm(
    mut anim: ResMut<HandAnim>,
    mut applied: MessageReader<EditApplied>,
    (mut eats, mut outcomes): (MessageReader<EatIntent>, MessageReader<UseOutcome>),
    hotbar: Res<Hotbar>,
    progress: Option<Res<super::removal::Progress>>,
    time: Res<Time>,
    (mut slot, mut asked): (Local<Option<usize>>, Local<Option<AskedToEat>>),
) {
    for edit in applied.read() {
        if matches!(edit.kind, AppliedKind::Placed(_)) {
            anim.swing();
        }
    }
    // The first frame only learns the slot: a World opening is not a switch.
    if slot
        .replace(hotbar.selected)
        .is_some_and(|was| was != hotbar.selected)
    {
        anim.switch();
    }
    let now = time.elapsed_secs_f64();
    if eats.read().last().is_some() {
        *asked = Some(AskedToEat {
            at_s: now,
            before: selected(&hotbar),
        });
    }
    if outcomes.read().any(|UseOutcome(outcome)| outcome.is_err()) {
        *asked = None;
    }
    if let Some(eat) = asked.as_ref() {
        let shrank = match (&eat.before, selected(&hotbar)) {
            (Some((was, count)), Some((item, now))) => *was == item && now < *count,
            (Some(_), None) => true,
            _ => false,
        };
        if shrank {
            anim.bite();
            *asked = None;
        } else if now - eat.at_s > EAT_PATIENCE_S {
            *asked = None;
        }
    }
    anim.set_mining(progress.is_some_and(|p| p.active()));
}

/// Whether the Player asked for stillness; what moves on screen checks this.
pub(super) fn still(preferences: &Option<Res<Preferences>>) -> bool {
    preferences.as_ref().is_some_and(|p| p.reduced_motion)
}

/// Advances the one state both views read.
pub(super) fn animate(mut anim: ResMut<HandAnim>, time: Res<Time>) {
    if anim.playing() {
        anim.tick(time.delta_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(anim: &mut HandAnim, seconds: f32) {
        let steps = (seconds / 0.01).round() as u32;
        for _ in 0..steps {
            anim.tick(0.01);
        }
    }

    #[test]
    fn the_arm_swings_forward_for_a_place_and_the_food_raises_it_to_the_mouth() {
        let mut anim = HandAnim::default();
        assert_eq!(anim.pose(false).arm_rad, 0.0, "at rest the arm hangs");
        anim.swing();
        run(&mut anim, SWING_S / 2.0);
        let swung = anim.pose(false).arm_rad;
        assert!((swung - ARM_SWING_RAD).abs() < 0.06, "{swung}");
        run(&mut anim, SWING_S);
        assert_eq!(anim.pose(false).arm_rad, 0.0, "and it returns");

        anim.bite();
        run(&mut anim, RISE_S);
        assert!(
            anim.pose(false).arm_rad > ARM_SWING_RAD * 1.9,
            "to the mouth"
        );
        let shrunk = (0..60).any(|_| {
            anim.tick(0.01);
            anim.pose(false).scale < 0.8
        });
        assert!(shrunk, "the food shrinks as it is bitten");
        run(&mut anim, BITE_S);
        assert_eq!(anim.pose(false), Pose::REST);
    }

    #[test]
    fn a_removal_chops_the_arm_over_and_over_and_reduced_motion_holds_it_still() {
        let mut anim = HandAnim::default();
        anim.set_mining(true);
        let mut peaks = 0;
        let mut last = 0.0;
        for _ in 0..200 {
            anim.tick(0.01);
            let arm = anim.pose(false).arm_rad;
            if last > arm && last > ARM_SWING_RAD * 0.9 {
                peaks += 1;
            }
            last = arm;
            assert_eq!(anim.pose(true), Pose::REST);
        }
        assert!(peaks >= 3, "chopped {peaks} times in two seconds");
    }

    #[test]
    fn a_switch_lowers_the_hand_and_raises_it_swapping_the_item_at_the_bottom() {
        let mut anim = HandAnim::default();
        anim.switch();
        assert!(!anim.swapped(), "the old item is still showing");
        run(&mut anim, SWITCH_S / 2.0 + 0.02);
        assert!(anim.swapped());
        assert!(
            anim.pose(false).offset.y < -SWITCH_DROP * 0.9,
            "lowest at the middle"
        );
        run(&mut anim, SWITCH_S);
        assert_eq!(anim.pose(false), Pose::REST);
        anim.bite();
        anim.switch();
        assert!(
            matches!(anim.motion, Some(Motion::Bite(_))),
            "a bite is not cut short"
        );
    }

    #[test]
    fn changing_the_selected_slot_starts_a_switch_but_opening_a_world_does_not() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<HandAnim>()
            .init_resource::<Hotbar>()
            .add_message::<EditApplied>()
            .add_message::<EatIntent>()
            .add_message::<UseOutcome>()
            .add_systems(Update, confirm);
        app.update();
        assert!(
            !app.world().resource::<HandAnim>().playing(),
            "the first frame learns the slot"
        );
        app.world_mut().resource_mut::<Hotbar>().selected = 3;
        app.update();
        assert!(app.world().resource::<HandAnim>().playing());
    }

    #[test]
    fn a_swing_reaches_out_then_returns_in_a_fifth_of_a_second() {
        let mut anim = HandAnim::default();
        assert_eq!(anim.pose(false), Pose::REST);
        anim.swing();
        run(&mut anim, SWING_S / 2.0);
        assert!(anim.pose(false).offset.z < -SWING_FORWARD * 0.95);
        run(&mut anim, SWING_S / 2.0 + 0.02);
        assert!(!anim.playing());
        assert_eq!(anim.pose(false), Pose::REST);
    }

    #[test]
    fn mining_repeats_until_it_stops_and_a_bite_is_not_cut_short_by_a_swing() {
        let mut anim = HandAnim::default();
        anim.set_mining(true);
        let mut reached = 0;
        let mut last = 0.0;
        for _ in 0..200 {
            anim.tick(0.01);
            let z = anim.pose(false).offset.z;
            if last > z && z < -SWING_FORWARD * 0.9 {
                reached += 1;
            }
            last = z;
        }
        assert!(reached >= 3, "swung {reached} times in two seconds");
        anim.set_mining(false);
        assert_eq!(anim.pose(false), Pose::REST);

        anim.bite();
        run(&mut anim, 0.3);
        anim.swing();
        assert!(matches!(anim.motion, Some(Motion::Bite(_))));
    }

    #[test]
    fn a_bite_lasts_a_second_shrinks_the_food_and_restores_it() {
        let mut anim = HandAnim::default();
        anim.bite();
        run(&mut anim, RISE_S);
        assert!(
            anim.pose(false).offset.distance(MOUTH) < 0.05,
            "at the mouth"
        );
        run(&mut anim, BITES as f32 * BITE_EACH_S - 0.01);
        assert!(anim.pose(false).scale < FOOD_LEFT + 0.1, "mostly eaten");
        run(&mut anim, LOWER_S + 0.02);
        assert!(!anim.playing(), "over in one second");
        assert_eq!(anim.pose(false).scale, 1.0);
        assert!((BITE_S - 1.0).abs() < f32::EPSILON);
    }

    fn confirming_app() -> App {
        use tesserae_core::inventory::Stack;
        let mut app = App::new();
        let mut hotbar = Hotbar::default();
        hotbar.stacks[0] = Some(Stack {
            item: "m:melon".into(),
            count: 3,
        });
        app.insert_resource(hotbar)
            .init_resource::<Time>()
            .init_resource::<HandAnim>()
            .add_message::<EditApplied>()
            .add_message::<EatIntent>()
            .add_message::<UseOutcome>()
            .add_systems(Update, confirm);
        app
    }

    fn count(app: &mut App, to: u32) {
        app.world_mut().resource_mut::<Hotbar>().stacks[0]
            .as_mut()
            .unwrap()
            .count = to;
    }

    fn biting(app: &App) -> bool {
        matches!(
            app.world().resource::<HandAnim>().motion,
            Some(Motion::Bite(_))
        )
    }

    #[test]
    fn a_bite_starts_only_when_the_stack_shrinks_after_an_eat_was_asked() {
        let mut app = confirming_app();
        app.world_mut().write_message(EatIntent);
        app.update();
        assert!(!biting(&app), "asking is not enough");
        count(&mut app, 2);
        app.update();
        assert!(biting(&app), "the Server took one");
    }

    #[test]
    fn a_refused_eat_never_bites_and_an_unasked_shrink_is_not_an_eat() {
        use crate::server::tool::UseRefusal;
        let mut app = confirming_app();
        app.world_mut().write_message(EatIntent);
        app.world_mut()
            .write_message(UseOutcome(Err(UseRefusal::NotHungry)));
        app.update();
        count(&mut app, 2);
        app.update();
        assert!(!biting(&app));
        count(&mut app, 1);
        app.update();
        assert!(!biting(&app), "shrinking alone, such as a Place, is no eat");
    }

    #[test]
    fn only_a_landed_place_swings_the_hand() {
        let mut app = confirming_app();
        app.world_mut().write_message(EditApplied {
            at: [0; 3],
            kind: AppliedKind::Removed(1),
        });
        app.update();
        assert!(app.world().resource::<HandAnim>().motion.is_none());
        app.world_mut().write_message(EditApplied {
            at: [0; 3],
            kind: AppliedKind::Placed(1),
        });
        app.update();
        assert!(matches!(
            app.world().resource::<HandAnim>().motion,
            Some(Motion::Swing(_))
        ));
    }

    #[test]
    fn reduced_motion_keeps_the_hand_at_rest() {
        let mut anim = HandAnim::default();
        anim.bite();
        run(&mut anim, 0.5);
        assert_eq!(anim.pose(true), Pose::REST);
    }
}
