//! One seeded Carry My Luggage episode.
//!
//! The scenario reads the hidden car and moves the operator. [`robot_follow`]
//! sees only the local sight and a cue. A component `TakeBag` can still set the
//! trip flag; [`EpisodeReport::main_passed`] does not read that flag.

use domain::{
    BagKind, Custody, Disk, EventKind, Id, Mm, Obstacle, ObstacleClass, Point, Pose, Step, World,
    segment_within, within_reach,
};
use navigation::{
    Cue, Follower, assume::ARRIVE_MM, assume::ROBOT_RADIUS_MM, assume::SENSOR_MM, assume::STEP_MM,
    look,
};
use sim_runtime::SimWorld;
use sim_runtime::assume::{
    BARRIER_RADIUS_MM, CLEARANCE_MM, HARD_RADIUS_MM, INSIDE_X_MAX_MM, PERSON_RADIUS_MM,
    SMALL_RADIUS_MM,
};
use task_3_1::{BACK, Effect, Event, LIMIT_MS, Phase, QUEUE};

use super::drive::{Drive, NavEnd, step_drive};
use super::{
    FeedError, HandoffEnd, PickEnd, confirm_ready, confirm_with_mock, execute_drop,
    execute_handoff, execute_pick, point_from_sample, stage_report,
};

pub const SOURCE_HASH: &str = "da69e2c276836ccb228ab3a9ba403b04efe38d7a1d30a7d4cbdcc490d83ce600";
pub const MODEL: &str = "m8-follow-car-v1";

const STEP_MS: u64 = 200;
const KEEP_MM: i32 = 1_500;
const GATE_MM: i32 = 1_500;
const STAY_STEPS: u32 = 3;
const ENTRANCE_X: i32 = 2_500;
const INSIDE_X: i32 = 800;

/// What the episode should attempt. The car point is scenario data.
#[derive(Clone, Copy, Debug)]
pub enum Script {
    Main {
        seed: u64,
        car: Option<Point>,
        then_return: bool,
        far_bag: bool,
    },
    EarlyCue {
        seed: u64,
    },
    StopShort {
        seed: u64,
    },
    Unseen {
        seed: u64,
    },
    Jump {
        seed: u64,
    },
    WrongBag {
        seed: u64,
    },
    Drop {
        seed: u64,
    },
    Cancel {
        seed: u64,
    },
    Stuck {
        seed: u64,
    },
    Budget {
        seed: u64,
    },
    Overflow,
    Loss {
        seed: u64,
    },
    Reversed {
        seed: u64,
    },
    WrongPerson {
        seed: u64,
    },
    OneBag {
        seed: u64,
    },
    BlockedHandoff {
        seed: u64,
    },
    HelpBonus {
        seed: u64,
    },
    DifferentQueue {
        seed: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Offer {
    AtCar,
    AtRouteEnd,
    Always,
}

#[derive(Clone, Debug)]
pub struct Sample {
    pub ms: u64,
    pub robot_x: i32,
    pub robot_y: i32,
    pub operator_x: i32,
    pub operator_y: i32,
}

#[derive(Clone, Debug)]
pub struct EpisodeReport {
    pub seed: u64,
    pub scenario: &'static str,
    pub source_hash: &'static str,
    pub model: &'static str,
    pub car: (i32, i32),
    pub robot: (i32, i32),
    pub operator: (i32, i32),
    pub pointed: u32,
    pub shown_bag: Option<u32>,
    pub custody: &'static str,
    pub phase: &'static str,
    pub clock_ms: u64,
    pub start_ms: u64,
    pub trip_flag: bool,
    pub main_passed: bool,
    pub local_cue: bool,
    pub followed_operator: bool,
    pub legal_motion: bool,
    pub longest_step_mm: i32,
    pub follow_moves: u32,
    pub returned: bool,
    pub queued: bool,
    pub queue_same: bool,
    pub crowd: usize,
    pub order: [ObstacleClass; 4],
    pub avoided: [bool; 4],
    pub start_inside: bool,
    pub start_fixed: bool,
    pub bags_ok: bool,
    pub bag_kind_ok: bool,
    pub car_hidden: bool,
    pub car_outside: bool,
    pub pickup_before_follow: bool,
    pub handoff_at_car: bool,
    pub grasped_pointed: bool,
    pub clocks_match: bool,
    pub trace_ok: bool,
    pub held_tracks: bool,
    pub approached: bool,
    pub cancelled: bool,
    pub moved_on_limit: bool,
    pub frozen_after_limit: bool,
    pub bonus_back: i32,
    pub bonus_queue: i32,
    pub help_used: bool,
    pub fail: Option<&'static str>,
    pub trace: Vec<Sample>,
}

impl EpisodeReport {
    pub fn summary(&self) -> String {
        let fail = self.fail.unwrap_or("-");
        format!(
            "model {MODEL}\nsource {SOURCE_HASH}\nseed {}\nscenario {}\ncar {},{}\nrobot {},{}\noperator {},{}\ncustody {}\nphase {}\nclock_ms {}\nstart_ms {}\nmain_passed {}\ntrip_flag {}\nlocal_cue {}\nfollow_moves {}\nlongest_step_mm {}\nlegal_motion {}\nreturned {}\nqueued {}\nqueue_same {}\ncrowd {}\nfail {fail}\ncomponent-cue-is-not-the-car\nassumption clearance-mm {CLEARANCE_MM}\nassumption robot-radius-mm {ROBOT_RADIUS_MM}\nassumption arrive-mm {ARRIVE_MM}\nassumption step-mm {STEP_MM}\nassumption hard-sensor-mm {}\nassumption sensor-mm {SENSOR_MM}\nassumption clock-start begin\nassumption obstacle-order seed-shuffle\nassumption few-return count={} stay-steps={STAY_STEPS}\nassumption outside-stand-in arena-half-plane\npublic-place-not-certified\nassumption size small={SMALL_RADIUS_MM} hard={HARD_RADIUS_MM} barrier={BARRIER_RADIUS_MM} person={PERSON_RADIUS_MM}\norder-not-required\n{}\n{}\ntrace_len {}\n",
            self.seed,
            self.scenario,
            self.car.0,
            self.car.1,
            self.robot.0,
            self.robot.1,
            self.operator.0,
            self.operator.1,
            self.custody,
            self.phase,
            self.clock_ms,
            self.start_ms,
            self.main_passed,
            self.trip_flag,
            self.local_cue,
            self.follow_moves,
            self.longest_step_mm,
            self.legal_motion,
            self.returned,
            self.queued,
            self.queue_same,
            self.crowd,
            navigation::assume::HARD_SENSOR_MM,
            self.crowd,
            open_lines(),
            hardware_lines(),
            self.trace.len(),
        )
    }
}

fn open_lines() -> String {
    task_3_1::OPEN
        .iter()
        .map(|id| format!("open {id} undecided"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn hardware_lines() -> String {
    ["CML-HW-OUT", "CML-HW-HIT", "CML-HW-SPL"]
        .iter()
        .map(|id| format!("hardware {id} not-run no-device"))
        .collect::<Vec<_>>()
        .join("\n")
}

struct Options {
    name: &'static str,
    seed: u64,
    start_ms: u64,
    car: Option<Point>,
    then_return: bool,
    far_bag: bool,
    hide_one: bool,
    offer: Offer,
    follow_crowd: bool,
    hold: bool,
    stop_short: bool,
    homeward: bool,
    jump: bool,
    wrong_bag: bool,
    drop: bool,
    cancel: bool,
    stuck: bool,
    budget: bool,
    overflow: bool,
    loss: bool,
    blocked_handoff: bool,
    help: bool,
    different_queue: bool,
}

impl Options {
    fn base(seed: u64, name: &'static str) -> Self {
        Self {
            name,
            seed,
            start_ms: 0,
            car: None,
            then_return: false,
            far_bag: false,
            hide_one: false,
            offer: Offer::AtCar,
            follow_crowd: false,
            hold: false,
            stop_short: false,
            homeward: false,
            jump: false,
            wrong_bag: false,
            drop: false,
            cancel: false,
            stuck: false,
            budget: false,
            overflow: false,
            loss: false,
            blocked_handoff: false,
            help: false,
            different_queue: false,
        }
    }
}

fn options(script: Script) -> Options {
    match script {
        Script::Main {
            seed,
            car,
            then_return,
            far_bag,
        } => {
            let name = if then_return {
                "main-return"
            } else if far_bag {
                "main-far-bag"
            } else {
                "main"
            };
            Options {
                name,
                seed,
                car,
                then_return,
                far_bag,
                ..Options::base(seed, name)
            }
        }
        Script::EarlyCue { seed } => Options {
            offer: Offer::Always,
            ..Options::base(seed, "early-cue")
        },
        Script::StopShort { seed } => Options {
            stop_short: true,
            offer: Offer::AtRouteEnd,
            ..Options::base(seed, "stop-short")
        },
        Script::Unseen { seed } => Options {
            hold: true,
            ..Options::base(seed, "unseen")
        },
        Script::Jump { seed } => Options {
            jump: true,
            ..Options::base(seed, "jump")
        },
        Script::WrongBag { seed } => Options {
            wrong_bag: true,
            ..Options::base(seed, "wrong-bag")
        },
        Script::Drop { seed } => Options {
            drop: true,
            ..Options::base(seed, "drop")
        },
        Script::Cancel { seed } => Options {
            cancel: true,
            ..Options::base(seed, "cancel")
        },
        Script::Stuck { seed } => Options {
            stuck: true,
            ..Options::base(seed, "stuck")
        },
        Script::Budget { seed } => Options {
            budget: true,
            ..Options::base(seed, "budget")
        },
        Script::Overflow => Options {
            overflow: true,
            start_ms: u64::MAX - 40,
            ..Options::base(0, "overflow")
        },
        Script::Loss { seed } => Options {
            loss: true,
            ..Options::base(seed, "loss")
        },
        Script::Reversed { seed } => Options {
            homeward: true,
            offer: Offer::AtRouteEnd,
            ..Options::base(seed, "reversed")
        },
        Script::WrongPerson { seed } => Options {
            follow_crowd: true,
            ..Options::base(seed, "wrong-person")
        },
        Script::OneBag { seed } => Options {
            hide_one: true,
            ..Options::base(seed, "one-bag")
        },
        Script::BlockedHandoff { seed } => Options {
            blocked_handoff: true,
            ..Options::base(seed, "blocked-handoff")
        },
        Script::HelpBonus { seed } => Options {
            then_return: true,
            help: true,
            ..Options::base(seed, "help-bonus")
        },
        Script::DifferentQueue { seed } => Options {
            then_return: true,
            different_queue: true,
            ..Options::base(seed, "different-queue")
        },
    }
}

struct Episode {
    opt: Options,
    world: SimWorld,
    trip: task_3_1::Trip,
    follower: Follower,
    pointed: Id,
    operator: Id,
    crowd_ids: Vec<Id>,
    order: [ObstacleClass; 4],
    route: Vec<Point>,
    route_i: usize,
    lane: i32,
    car: Point,
    start_pose: (i32, i32),
    local_cue: bool,
    followed_operator: bool,
    legal_motion: bool,
    longest_step_mm: i32,
    follow_moves: u32,
    returned: bool,
    queued: bool,
    queue_same: bool,
    present: [bool; 4],
    hit: [bool; 4],
    start_fixed: bool,
    bags_ok: bool,
    bag_kind_ok: bool,
    pickup_before_follow: bool,
    handoff_at_car: bool,
    grasped_pointed: bool,
    near_at_handoff: bool,
    clocks_match: bool,
    trace_ok: bool,
    held_tracks: bool,
    approached: bool,
    cancelled: bool,
    moved_on_limit: bool,
    frozen_after_limit: bool,
    jumped: bool,
    shown_bag: Option<u32>,
    fail: Option<&'static str>,
    trace: Vec<Sample>,
    last_ms: Option<u64>,
    outbound: bool,
}

pub fn run(script: Script) -> EpisodeReport {
    let mut episode = Episode::prepare(options(script));
    episode.execute();
    episode.report()
}

impl Episode {
    fn prepare(opt: Options) -> Self {
        let mut world = SimWorld::from_seed(opt.seed);
        if let Some(car) = opt.car {
            world.set_car(car);
        }
        let start_pose = xy(world.observe().robot.pose.point());
        let operator = world.observe().operator.id;
        let pointed = world.observe().bags[(opt.seed % 2) as usize].id;
        let order = layout_order(&world);
        let crowd_ids = crowd_ids(&world);
        let car = world.truth().car();
        Self {
            opt,
            world,
            trip: task_3_1::Trip::new(),
            follower: Follower::follow(operator),
            pointed,
            operator,
            crowd_ids,
            order,
            route: Vec::new(),
            route_i: 0,
            lane: 0,
            car,
            start_pose,
            local_cue: false,
            followed_operator: false,
            legal_motion: true,
            longest_step_mm: 0,
            follow_moves: 0,
            returned: false,
            queued: false,
            queue_same: true,
            present: [false; 4],
            hit: [false; 4],
            start_fixed: false,
            bags_ok: false,
            bag_kind_ok: false,
            pickup_before_follow: false,
            handoff_at_car: false,
            grasped_pointed: false,
            near_at_handoff: false,
            clocks_match: true,
            trace_ok: true,
            held_tracks: true,
            approached: false,
            cancelled: false,
            moved_on_limit: false,
            frozen_after_limit: false,
            jumped: false,
            shown_bag: None,
            fail: None,
            trace: Vec::new(),
            last_ms: None,
            outbound: true,
        }
    }

    fn execute(&mut self) {
        if self.begin().is_err() {
            return;
        }
        self.mark_setup();
        if self.opt.overflow {
            self.fail = Some("overflow");
            return;
        }
        if !self.bags_ok || !self.bag_kind_ok {
            self.fail = Some("setup");
            return;
        }
        if self.perceive_and_ready().is_err() {
            return;
        }
        if self.opt.drop {
            let _ = self.drop_held();
            return;
        }
        if self.opt.budget {
            self.budget_probe();
            return;
        }
        if self.opt.jump {
            self.jump_to_car();
            return;
        }
        if self.opt.homeward {
            self.move_to_car_for_reverse();
        }
        if self.fail.is_some() {
            return;
        }
        self.prepare_route();
        if self.opt.stuck {
            self.seal_robot();
        }
        self.follow_out();
        if self.opt.then_return && self.main_geometry() {
            self.return_with_queue();
        }
    }

    fn begin(&mut self) -> Result<(), ()> {
        if self.opt.start_ms > 0
            && self
                .world
                .advance(Step::from_ms(self.opt.start_ms))
                .is_err()
        {
            self.fail = Some("overflow");
            return Err(());
        }
        if self.trip.apply(self.opt.start_ms, Event::Begin).is_err() {
            self.fail = Some("begin");
            return Err(());
        }
        self.start_fixed = xy(self.world.observe().robot.pose.point()) == self.start_pose;
        self.note_clocks();
        self.sample();
        Ok(())
    }

    fn mark_setup(&mut self) {
        if self.opt.far_bag {
            let at = point(0, 3_000);
            if self.world.place_free(self.pointed, at).is_err() {
                self.fail = Some("setup");
            }
        }
        if self.opt.hide_one {
            let other = self
                .world
                .observe()
                .bags
                .iter()
                .find(|bag| bag.id != self.pointed)
                .map(|bag| bag.id);
            if let Some(other) = other {
                let _ = self.world.set_visible(other, false);
            }
        }
        let observed = self.world.observe();
        let operator_at = observed.operator.pose.point();
        let visible = observed
            .bags
            .iter()
            .filter(|bag| bag.visible && within_reach(operator_at, bag.pose.point(), 2_000))
            .count();
        self.bags_ok = visible >= 2;
        self.bag_kind_ok = observed
            .bags
            .iter()
            .all(|bag| bag.kind == BagKind::LightweightStiffVerticalHandles)
            && !observed.bags.is_empty();
        if self.opt.hold {
            self.world.place_operator(self.car);
        }
        self.present = [false; 4];
        for obstacle in &observed.obstacles {
            if obstacle.pose.point().x().get() > INSIDE_X_MAX_MM {
                self.present[class_index(obstacle.class)] = true;
            }
        }
    }

    fn perceive_and_ready(&mut self) -> Result<(), ()> {
        let observed = self.world.observe();
        let sample = match self.world.aim_at(self.pointed) {
            Ok(sample) => sample,
            Err(_) => {
                self.fail = Some("aim");
                return Err(());
            }
        };
        let chosen = match point_from_sample(
            Some(sample.origin()),
            Some((sample.dx(), sample.dy())),
            &observed.bags,
        ) {
            Ok(id) => id,
            Err(_) => {
                self.fail = Some("select");
                return Err(());
            }
        };
        if chosen != self.pointed {
            self.fail = Some("select");
            return Err(());
        }
        let stamp = self.must_stamp()?;
        let operator = self.operator;
        let effect = self
            .commit(stamp, move |world, trip| {
                let _ = world;
                stage_report(trip, stamp, operator, chosen)
            })
            .map_err(|reason| {
                self.fail = Some(reason);
            })?;
        let stamp = self.must_stamp()?;
        let named = self
            .commit(stamp, |world, trip| {
                let _ = world;
                confirm_with_mock(trip, stamp, effect)
            })
            .map_err(|reason| {
                self.fail = Some(reason);
            })?;
        self.shown_bag = Some(named.get());
        self.approach()?;
        if self.opt.wrong_bag {
            let other = self
                .world
                .observe()
                .bags
                .iter()
                .find(|bag| bag.id != self.pointed)
                .map(|bag| bag.id)
                .ok_or_else(|| {
                    self.fail = Some("wrong-bag");
                })?;
            let stamp = self.must_stamp()?;
            let end = self
                .commit(stamp, |world, trip| {
                    execute_pick(world, trip, stamp, Effect::Pick { bag: other })
                })
                .map_err(|reason| {
                    self.fail = Some(reason);
                })?;
            if !matches!(end, PickEnd::Mismatched { .. }) {
                self.fail = Some("wrong-bag");
                return Err(());
            }
            self.fail = Some("wrong-bag");
            return Err(());
        }
        let stamp = self.must_stamp()?;
        let pointed = self.pointed;
        let end = self
            .commit(stamp, move |world, trip| {
                execute_pick(world, trip, stamp, Effect::Pick { bag: pointed })
            })
            .map_err(|reason| {
                self.fail = Some(reason);
            })?;
        let PickEnd::Matched { signal, bag } = end else {
            self.fail = Some("grasp");
            return Err(());
        };
        if bag != self.pointed {
            self.fail = Some("grasp");
            return Err(());
        }
        self.grasped_pointed = true;
        self.pickup_before_follow = true;
        self.check_hold();
        let stamp = self.must_stamp()?;
        self.commit(stamp, |world, trip| {
            confirm_ready(world, trip, stamp, signal)
        })
        .map_err(|reason| {
            self.fail = Some(reason);
        })?;
        if self.trip.phase() != Phase::Following {
            self.fail = Some("ready");
            return Err(());
        }
        Ok(())
    }

    fn approach(&mut self) -> Result<(), ()> {
        self.follower = Follower::follow(self.operator);
        let start = xy(self.world.observe().robot.pose.point());
        for _ in 0..800 {
            let stamp = self.must_stamp()?;
            let before = xy(self.world.observe().robot.pose.point());
            let end = self
                .drive(stamp, Drive::Approach { bag: self.pointed }, Cue::None)
                .map_err(|reason| {
                    self.fail = Some(reason);
                })?;
            let after = xy(self.world.observe().robot.pose.point());
            if after != before {
                self.approached = true;
            }
            match end {
                NavEnd::InReach => {
                    self.approached = after != start || self.approached;
                    return Ok(());
                }
                NavEnd::Moved { .. } | NavEnd::Blocked | NavEnd::Waiting => {}
                NavEnd::Stuck => {
                    self.fail = Some("stuck");
                    return Err(());
                }
                _ => {
                    self.fail = Some("approach");
                    return Err(());
                }
            }
        }
        self.fail = Some("approach");
        Err(())
    }

    fn drop_held(&mut self) -> Result<(), ()> {
        let stamp = self.must_stamp()?;
        let pointed = self.pointed;
        self.commit(stamp, move |world, trip| {
            execute_drop(world, trip, stamp, pointed)
        })
        .map_err(|reason| {
            self.fail = Some(reason);
        })?;
        self.fail = Some("dropped");
        Ok(())
    }

    fn budget_probe(&mut self) {
        self.follower = Follower::follow(self.operator);
        let limit = self.opt.start_ms.saturating_add(LIMIT_MS);
        let before = xy(self.world.observe().robot.pose.point());
        let moved = self.drive(
            limit,
            Drive::Follow {
                person: self.operator,
            },
            Cue::None,
        );
        let after = xy(self.world.observe().robot.pose.point());
        self.moved_on_limit = moved.is_ok() && after != before && self.world.now_ms() == limit;
        let frozen = after;
        let late = self.drive(
            limit.saturating_add(1),
            Drive::Follow {
                person: self.operator,
            },
            Cue::TakeBag,
        );
        self.frozen_after_limit = late.as_ref().is_err_and(|reason| *reason == "time-up")
            && xy(self.world.observe().robot.pose.point()) == frozen;
        self.fail = Some("time-up");
    }

    fn jump_to_car(&mut self) {
        self.world.set_obstacles(Vec::new());
        self.world.place_operator(self.car);
        let before = xy(self.world.observe().robot.pose.point());
        if self.world.try_slide(self.car, ROBOT_RADIUS_MM).is_err() {
            self.fail = Some("jump");
            self.jumped = true;
            self.legal_motion = false;
            return;
        }
        self.note_step(before, xy(self.world.observe().robot.pose.point()), false);
        self.jumped = true;
        self.legal_motion = false;
        let stamp = match self.legal_stamp() {
            Some(stamp) => stamp,
            None => {
                self.fail = Some("time-up");
                return;
            }
        };
        if self
            .commit(stamp, |world, trip| {
                let _ = world;
                trip.apply(stamp, Event::Arrived)
                    .map(|_| ())
                    .map_err(FeedError::Trip)
            })
            .is_err()
        {
            self.fail = Some("jump");
            return;
        }
        let _ = self.hand_off();
        if self.fail.is_none() {
            self.fail = Some("jump");
        }
    }

    fn move_to_car_for_reverse(&mut self) {
        self.world.set_obstacles(Vec::new());
        self.world.place_operator(self.car);
        let before = xy(self.world.observe().robot.pose.point());
        if self.world.try_slide(self.car, ROBOT_RADIUS_MM).is_ok() {
            self.note_step(before, xy(self.world.observe().robot.pose.point()), false);
        }
        self.jumped = true;
        self.legal_motion = false;
    }

    fn prepare_route(&mut self) {
        let from = self.world.observe().operator.pose.point();
        let goal = if self.opt.homeward {
            point(0, 1_000)
        } else {
            self.car
        };
        let obstacles = self.world.observe().obstacles;
        let (route, lane) = route_to(from, goal, &obstacles, self.opt.seed);
        self.lane = lane;
        self.route = if self.opt.stop_short {
            let mut route = route;
            while route.last().is_some_and(|item| *item == self.car) {
                route.pop();
            }
            if route.is_empty() {
                route.push(from);
            }
            route
        } else {
            route
        };
        self.route_i = 0;
    }

    fn seal_robot(&mut self) {
        let robot = self.world.observe().robot.pose.point();
        let mut obstacles = self.world.observe().obstacles;
        let dirs = [
            (400, 0),
            (-400, 0),
            (0, 400),
            (0, -400),
            (280, 280),
            (-280, 280),
            (280, -280),
            (-280, -280),
        ];
        for (index, (dx, dy)) in dirs.iter().enumerate() {
            let center = point(
                robot.x().get().saturating_add(*dx),
                robot.y().get().saturating_add(*dy),
            );
            let id = Id::new(40 + u32::try_from(index).unwrap_or(1)).unwrap_or(self.operator);
            obstacles.push(Obstacle {
                id,
                pose: Pose::new(center),
                class: ObstacleClass::SmallGround,
                clearance: Mm::new(CLEARANCE_MM),
                bodies: vec![Disk {
                    id,
                    center,
                    radius: Mm::new(200),
                }],
            });
        }
        self.world.set_obstacles(obstacles);
    }

    fn follow_out(&mut self) {
        let person = if self.opt.follow_crowd {
            self.crowd_ids.first().copied().unwrap_or(self.operator)
        } else {
            self.operator
        };
        self.followed_operator = person == self.operator;
        self.follower = Follower::follow(person);
        let mut lost_run = 0u32;
        let mut loss_done = false;
        for _ in 0..4_000 {
            if self.opt.loss && !loss_done && self.follow_moves >= 2 {
                loss_done = true;
                let back = self
                    .route
                    .get(self.route_i)
                    .copied()
                    .unwrap_or(self.world.observe().operator.pose.point());
                let robot = xy(self.world.observe().robot.pose.point());
                self.world
                    .place_operator(point(robot.0, robot.1.saturating_add(3_000)));
                let Some(stamp) = self.legal_stamp() else {
                    self.fail = Some("time-up");
                    break;
                };
                let _ = self.drive(stamp, Drive::Follow { person }, Cue::None);
                self.world.place_operator(back);
                let Some(stamp) = self.legal_stamp() else {
                    self.fail = Some("time-up");
                    break;
                };
                let _ = self.drive(stamp, Drive::Follow { person }, Cue::None);
                continue;
            }
            if self.opt.cancel && self.follow_moves >= 2 {
                let Some(stamp) = self.legal_stamp() else {
                    self.fail = Some("time-up");
                    break;
                };
                if self.drive(stamp, Drive::Cancel, Cue::None).is_ok() {
                    self.cancelled = true;
                    self.fail = Some("cancel");
                }
                break;
            }
            if !self.opt.hold {
                self.advance_operator();
            }
            let cue = self.cue_for_robot();
            let Some(stamp) = self.legal_stamp() else {
                self.fail = Some("time-up");
                break;
            };
            let end = match self.drive(stamp, Drive::Follow { person }, cue) {
                Ok(end) => end,
                Err("time-up") => {
                    self.fail = Some("time-up");
                    break;
                }
                Err(reason) => {
                    self.fail = Some(reason);
                    break;
                }
            };
            if matches!(end, NavEnd::Moved { .. }) {
                self.follow_moves = self.follow_moves.saturating_add(1);
                self.check_hold();
            }
            match end {
                NavEnd::Arrived { .. } => {
                    if cue == Cue::TakeBag {
                        self.local_cue = true;
                    }
                    if self.hand_off().is_err() {
                        return;
                    }
                    break;
                }
                NavEnd::Stuck => {
                    self.fail = Some("stuck");
                    break;
                }
                NavEnd::WrongPerson => {
                    self.fail = Some("wrong-person");
                    break;
                }
                NavEnd::Lost => {
                    lost_run = lost_run.saturating_add(1);
                    if lost_run > 6 {
                        self.fail = Some(if self.opt.hold { "unseen" } else { "lost" });
                        break;
                    }
                }
                NavEnd::Cancelled => {
                    self.cancelled = true;
                    self.fail = Some("cancel");
                    break;
                }
                _ => lost_run = 0,
            }
        }
        if self.fail.is_none() && !self.trip.main_complete() && !self.opt.then_return {
            self.fail = Some("not at the car");
        }
    }

    fn advance_operator(&mut self) {
        if self.route_i + 1 >= self.route.len() {
            return;
        }
        let robot = self.world.observe().robot.pose.point();
        let operator = self.world.observe().operator.pose.point();
        if !within_reach(robot, operator, KEEP_MM) {
            return;
        }
        self.route_i += 1;
        let next = self.route[self.route_i];
        self.world.place_operator(next);
    }

    fn cue_for_robot(&mut self) -> Cue {
        let observed = self.world.observe();
        let (people, _) = look(
            observed.robot.pose.point(),
            observed.operator,
            &observed.obstacles,
        );
        let sees = people.iter().any(|person| person.id == self.operator);
        let operator = observed.operator.pose.point();
        let offer = match self.opt.offer {
            Offer::Always => true,
            Offer::AtCar => operator == self.car,
            Offer::AtRouteEnd => self
                .route
                .last()
                .is_some_and(|last| operator == *last && self.route_i + 1 >= self.route.len()),
        };
        if offer && sees && !self.opt.follow_crowd {
            Cue::TakeBag
        } else {
            Cue::None
        }
    }

    fn hand_off(&mut self) -> Result<(), ()> {
        if self.opt.blocked_handoff {
            self.insert_handoff_disk();
        }
        let Some(stamp) = self.legal_stamp() else {
            self.fail = Some("time-up");
            return Err(());
        };
        let operator = self.world.observe().operator.pose.point();
        let robot = self.world.observe().robot.pose.point();
        let operator_id = self.operator;
        let end = self
            .commit(stamp, move |world, trip| {
                execute_handoff(world, trip, stamp, operator_id)
            })
            .map_err(|reason| {
                self.fail = Some(if self.opt.blocked_handoff && reason == "blocked" {
                    "blocked-handoff"
                } else {
                    reason
                });
            })?;
        match end {
            HandoffEnd::Released { bag, .. } if bag == self.pointed => {
                self.handoff_at_car = operator == self.car && self.car.x().get() > INSIDE_X_MAX_MM;
                self.near_at_handoff = within_reach(robot, operator, ARRIVE_MM);
            }
            HandoffEnd::Failed { .. } => {
                self.fail = Some("blocked-handoff");
                return Err(());
            }
            _ => {
                self.fail = Some("handoff");
                return Err(());
            }
        }
        Ok(())
    }

    fn insert_handoff_disk(&mut self) {
        let robot = self.world.observe().robot.pose.point();
        let operator = self.world.observe().operator.pose.point();
        let mx = i32::try_from((i64::from(robot.x().get()) + i64::from(operator.x().get())) / 2)
            .unwrap_or(0);
        let my = i32::try_from((i64::from(robot.y().get()) + i64::from(operator.y().get())) / 2)
            .unwrap_or(0);
        let center = point(mx, my);
        let mut obstacles = self.world.observe().obstacles;
        let id = Id::new(77).unwrap_or(self.operator);
        obstacles.push(Obstacle {
            id,
            pose: Pose::new(center),
            class: ObstacleClass::HardToSee,
            clearance: Mm::new(CLEARANCE_MM),
            bodies: vec![Disk {
                id,
                center,
                radius: Mm::new(60),
            }],
        });
        self.world.set_obstacles(obstacles);
    }

    fn return_with_queue(&mut self) {
        self.outbound = false;
        if self.crowd_ids.is_empty() {
            return;
        }
        let robot = self.world.observe().robot.pose.point();
        let tail0 = point(robot.x().get(), robot.y().get().saturating_add(1_000));
        let mut path = vec![tail0];
        extend(&mut path, point(tail0.x().get(), self.lane));
        extend(&mut path, point(ENTRANCE_X, self.lane));
        extend(&mut path, point(INSIDE_X, self.lane));
        let entrance = point(ENTRANCE_X, self.lane);
        let original = self.crowd_ids.clone();
        self.place_queue(tail0);
        let tail = crowd_ids(&self.world)
            .first()
            .copied()
            .unwrap_or(self.operator);
        self.follower = Follower::follow(tail);
        let mut index = 0usize;
        let mut inside_run = 0u32;
        let mut was_outside = robot.x().get() > INSIDE_X_MAX_MM;
        let mut crossed = false;
        let mut near_gate = false;
        for _ in 0..4_000 {
            let current = path[index];
            let robot_now = self.world.observe().robot.pose.point();
            if within_reach(robot_now, current, KEEP_MM) && index + 1 < path.len() {
                index += 1;
                self.place_queue(path[index]);
            }
            let Some(stamp) = self.legal_stamp() else {
                break;
            };
            let before = xy(robot_now);
            let end = match self.drive(stamp, Drive::Accompany { person: tail }, Cue::None) {
                Ok(end) => end,
                Err(_) => break,
            };
            let robot_after = self.world.observe().robot.pose.point();
            let after = xy(robot_after);
            if was_outside && after.0 <= INSIDE_X_MAX_MM {
                crossed = true;
            }
            if after.0 > INSIDE_X_MAX_MM {
                was_outside = true;
            }
            if within_reach(robot_after, entrance, GATE_MM) {
                near_gate = true;
            }
            let tail_done = index + 1 >= path.len();
            if tail_done
                && after.0 <= INSIDE_X_MAX_MM
                && within_reach(robot_after, path[index], GATE_MM)
            {
                inside_run = inside_run.saturating_add(1);
            }
            let _ = (before, end);
            if inside_run >= STAY_STEPS && crossed && near_gate {
                break;
            }
            if matches!(end, NavEnd::Stuck | NavEnd::Lost | NavEnd::WrongPerson) {
                break;
            }
        }
        self.returned = crossed
            && near_gate
            && xy(self.world.observe().robot.pose.point()).0 <= INSIDE_X_MAX_MM;
        self.queue_same = crowd_ids(&self.world) == original;
        self.queued = self.returned && inside_run >= STAY_STEPS && self.queue_same;
    }

    fn place_queue(&mut self, tail: Point) {
        let mut obstacles = self.world.observe().obstacles;
        for obstacle in &mut obstacles {
            if obstacle.class != ObstacleClass::Crowd {
                continue;
            }
            for (index, body) in obstacle.bodies.iter_mut().enumerate() {
                if self.opt.different_queue
                    && let Some(id) = Id::new(30 + u32::try_from(index).unwrap_or(0))
                {
                    body.id = id;
                }
                let y = tail
                    .y()
                    .get()
                    .saturating_add(i32::try_from(index).unwrap_or(0).saturating_mul(600));
                body.center = point(tail.x().get(), y);
            }
            obstacle.pose = Pose::new(tail);
        }
        self.world.set_obstacles(obstacles);
    }

    fn main_geometry(&self) -> bool {
        self.handoff_at_car
            && self.near_at_handoff
            && self.local_cue
            && self.legal_motion
            && self.follow_moves > 0
            && self.followed_operator
            && !self.jumped
            && self.grasped_pointed
            && self.pickup_before_follow
            && self.held_tracks
            && self.clocks_match
            && self.trace_ok
            && self.bags_ok
            && self.bag_kind_ok
            && self.start_fixed
            && self.start_pose.0 <= INSIDE_X_MAX_MM
            && self.car.x().get() > INSIDE_X_MAX_MM
            && self.world.revealed_car().is_none()
            && self.custody_ok()
            && self.world.now_ms().saturating_sub(self.opt.start_ms) <= LIMIT_MS
    }

    fn custody_ok(&self) -> bool {
        self.world.observe().bags.iter().any(|bag| {
            bag.id == self.pointed
                && bag.custody == Custody::Operator
                && bag.pose.point() == self.car
        })
    }

    fn report(&mut self) -> EpisodeReport {
        let passed = self.main_geometry();
        if passed {
            self.fail = None;
        } else if self.fail.is_none() {
            self.fail = Some("not at the car");
        }
        let observed = self.world.observe();
        let bonus_back = if self.returned {
            task_3_1::bonus_line(BACK, self.opt.help)
        } else {
            0
        };
        let bonus_queue = if self.queued {
            task_3_1::bonus_line(QUEUE, self.opt.help)
        } else {
            0
        };
        EpisodeReport {
            seed: self.opt.seed,
            scenario: self.opt.name,
            source_hash: SOURCE_HASH,
            model: MODEL,
            car: xy(self.car),
            robot: xy(observed.robot.pose.point()),
            operator: xy(observed.operator.pose.point()),
            pointed: self.pointed.get(),
            shown_bag: self.shown_bag,
            custody: custody_name(&observed, self.pointed),
            phase: phase_name(self.trip.phase()),
            clock_ms: self.world.now_ms(),
            start_ms: self.opt.start_ms,
            trip_flag: self.trip.main_complete(),
            main_passed: passed,
            local_cue: self.local_cue,
            followed_operator: self.followed_operator,
            legal_motion: self.legal_motion,
            longest_step_mm: self.longest_step_mm,
            follow_moves: self.follow_moves,
            returned: self.returned,
            queued: self.queued,
            queue_same: self.queue_same,
            crowd: self.crowd_ids.len(),
            order: self.order,
            avoided: [
                self.present[0] && !self.hit[0],
                self.present[1] && !self.hit[1],
                self.present[2] && !self.hit[2],
                self.present[3] && !self.hit[3],
            ],
            start_inside: self.start_pose.0 <= INSIDE_X_MAX_MM,
            start_fixed: self.start_fixed,
            bags_ok: self.bags_ok,
            bag_kind_ok: self.bag_kind_ok,
            car_hidden: self.world.revealed_car().is_none(),
            car_outside: self.car.x().get() > INSIDE_X_MAX_MM,
            pickup_before_follow: self.pickup_before_follow,
            handoff_at_car: self.handoff_at_car,
            grasped_pointed: self.grasped_pointed,
            clocks_match: self.clocks_match && self.trip.now_ms() == Some(self.world.now_ms()),
            trace_ok: self.trace_ok,
            held_tracks: self.held_tracks,
            approached: self.approached,
            cancelled: self.cancelled,
            moved_on_limit: self.moved_on_limit,
            frozen_after_limit: self.frozen_after_limit,
            bonus_back,
            bonus_queue,
            help_used: self.opt.help,
            fail: self.fail,
            trace: std::mem::take(&mut self.trace),
        }
    }

    fn commit<T>(
        &mut self,
        at_ms: u64,
        f: impl FnOnce(&mut SimWorld, &mut task_3_1::Trip) -> Result<T, FeedError>,
    ) -> Result<T, &'static str> {
        if at_ms < self.world.now_ms() {
            return Err("backdate");
        }
        let saved_world = self.world.clone();
        let saved_trip = self.trip;
        let delta = at_ms - self.world.now_ms();
        if self.world.advance(Step::from_ms(delta)).is_err() {
            self.world = saved_world;
            return Err("overflow");
        }
        let before = xy(saved_world.observe().robot.pose.point());
        match f(&mut self.world, &mut self.trip) {
            Ok(value) => {
                if self.trip.now_ms() != Some(at_ms) || self.world.now_ms() != at_ms {
                    self.world = saved_world;
                    self.trip = saved_trip;
                    self.clocks_match = false;
                    return Err("split");
                }
                let after = xy(self.world.observe().robot.pose.point());
                if after != before {
                    self.note_step(before, after, self.outbound);
                }
                self.sample();
                Ok(value)
            }
            Err(FeedError::TimeUp) => {
                if self.world.now_ms() != at_ms {
                    self.clocks_match = false;
                }
                self.sample();
                Err("time-up")
            }
            Err(error) => {
                self.world = saved_world;
                self.trip = saved_trip;
                Err(feed_name(error))
            }
        }
    }

    /// One robot step. The car coordinate is not an argument.
    fn drive(&mut self, at_ms: u64, drive: Drive, cue: Cue) -> Result<NavEnd, &'static str> {
        let before_clock = self.world.now_ms();
        let before = xy(self.world.observe().robot.pose.point());
        let result = robot_follow(
            &mut self.world,
            &mut self.trip,
            &mut self.follower,
            at_ms,
            drive,
            cue,
        );
        match result {
            Ok(end) => {
                if self.trip.now_ms() != Some(self.world.now_ms()) {
                    self.clocks_match = false;
                }
                let after = xy(self.world.observe().robot.pose.point());
                if after != before {
                    self.note_step(before, after, self.outbound);
                }
                self.sample();
                Ok(end)
            }
            Err(FeedError::TimeUp) => {
                if self.trip.now_ms() != Some(self.world.now_ms()) {
                    self.clocks_match = false;
                }
                self.sample();
                Err("time-up")
            }
            Err(error) => {
                if self.world.now_ms() != before_clock {
                    self.clocks_match = false;
                }
                Err(feed_name(error))
            }
        }
    }

    fn legal_stamp(&self) -> Option<u64> {
        let now = self.world.now_ms();
        let limit = self.opt.start_ms.checked_add(LIMIT_MS)?;
        if now >= limit {
            return None;
        }
        let step = STEP_MS.min(limit - now);
        now.checked_add(step)
    }

    fn must_stamp(&mut self) -> Result<u64, ()> {
        match self.legal_stamp() {
            Some(stamp) => Ok(stamp),
            None => {
                self.fail = Some(if self.opt.overflow {
                    "overflow"
                } else {
                    "time-up"
                });
                Err(())
            }
        }
    }

    fn note_step(&mut self, from: (i32, i32), to: (i32, i32), avoid: bool) {
        let len = ceil_dist(from.0, from.1, to.0, to.1);
        if len > self.longest_step_mm {
            self.longest_step_mm = len;
        }
        if len > STEP_MM {
            self.legal_motion = false;
        }
        if avoid {
            let hits = body_hits(
                point(from.0, from.1),
                point(to.0, to.1),
                &self.world.observe().obstacles,
            );
            for (hit, slot) in hits.iter().zip(self.hit.iter_mut()) {
                if *hit {
                    *slot = true;
                }
            }
        }
    }

    fn sample(&mut self) {
        let ms = self.world.now_ms();
        if self.last_ms.is_some_and(|prev| ms < prev) {
            self.trace_ok = false;
        }
        self.last_ms = Some(ms);
        if self.trip.now_ms().is_some() && self.trip.now_ms() != Some(ms) {
            self.clocks_match = false;
        }
        let observed = self.world.observe();
        self.trace.push(Sample {
            ms,
            robot_x: observed.robot.pose.point().x().get(),
            robot_y: observed.robot.pose.point().y().get(),
            operator_x: observed.operator.pose.point().x().get(),
            operator_y: observed.operator.pose.point().y().get(),
        });
    }

    fn note_clocks(&mut self) {
        if self.trip.now_ms() != Some(self.world.now_ms()) {
            self.clocks_match = false;
        }
    }

    fn check_hold(&mut self) {
        let Some(id) = self.world.carried() else {
            return;
        };
        let observed = self.world.observe();
        let tracks = observed.bags.iter().any(|bag| {
            bag.id == id
                && bag.custody == Custody::Gripper
                && bag.pose.point() == observed.robot.pose.point()
        });
        if !tracks {
            self.held_tracks = false;
        }
    }
}

/// Robot control. `cue` is the only end signal. The car is not an argument.
fn robot_follow(
    world: &mut SimWorld,
    trip: &mut task_3_1::Trip,
    follower: &mut Follower,
    at_ms: u64,
    drive: Drive,
    cue: Cue,
) -> Result<NavEnd, FeedError> {
    debug_assert!(world.revealed_car().is_none());
    step_drive(world, trip, follower, at_ms, drive, cue)
}

fn point(x: i32, y: i32) -> Point {
    Point::new(Mm::new(x), Mm::new(y))
}

fn xy(at: Point) -> (i32, i32) {
    (at.x().get(), at.y().get())
}

fn layout_order(world: &SimWorld) -> [ObstacleClass; 4] {
    world
        .events()
        .iter()
        .find_map(|event| match event.kind {
            EventKind::Layout { order, .. } => Some(order),
            _ => None,
        })
        .unwrap_or([
            ObstacleClass::SmallGround,
            ObstacleClass::HardToSee,
            ObstacleClass::Crowd,
            ObstacleClass::Barriers,
        ])
}

fn crowd_ids(world: &SimWorld) -> Vec<Id> {
    world
        .observe()
        .obstacles
        .iter()
        .filter(|obstacle| obstacle.class == ObstacleClass::Crowd)
        .flat_map(|obstacle| obstacle.bodies.iter().map(|body| body.id))
        .collect()
}

fn class_index(class: ObstacleClass) -> usize {
    match class {
        ObstacleClass::SmallGround => 0,
        ObstacleClass::HardToSee => 1,
        ObstacleClass::Crowd => 2,
        ObstacleClass::Barriers => 3,
    }
}

fn route_to(from: Point, goal: Point, obstacles: &[Obstacle], seed: u64) -> (Vec<Point>, i32) {
    let lane = lane_y(obstacles, seed);
    let drop = drop_x(goal, lane, obstacles);
    let mut points = vec![from];
    extend(&mut points, point(from.x().get(), lane));
    extend(&mut points, point(drop, lane));
    extend(&mut points, point(drop, goal.y().get()));
    extend(&mut points, goal);
    (points, lane)
}

fn lane_y(obstacles: &[Obstacle], seed: u64) -> i32 {
    let mut y = 0i32;
    for obstacle in obstacles {
        for disk in &obstacle.bodies {
            let need = disk
                .center
                .y()
                .get()
                .saturating_add(disk.radius.get())
                .saturating_add(ROBOT_RADIUS_MM)
                .saturating_add(1);
            y = y.max(need);
        }
    }
    y.saturating_add(i32::try_from(seed % 4).unwrap_or(0) * 250)
}

fn drop_x(goal: Point, lane: i32, obstacles: &[Obstacle]) -> i32 {
    if vertical_clear(goal.x().get(), lane, goal.y().get(), obstacles) {
        goal.x().get()
    } else {
        let mut x = goal.x().get();
        for obstacle in obstacles {
            for disk in &obstacle.bodies {
                let edge = disk
                    .center
                    .x()
                    .get()
                    .saturating_add(disk.radius.get())
                    .saturating_add(ROBOT_RADIUS_MM)
                    .saturating_add(1);
                x = x.max(edge);
            }
        }
        x
    }
}

fn vertical_clear(x: i32, y0: i32, y1: i32, obstacles: &[Obstacle]) -> bool {
    !body_hits(point(x, y0), point(x, y1), obstacles)
        .iter()
        .any(|hit| *hit)
}

fn body_hits(from: Point, to: Point, obstacles: &[Obstacle]) -> [bool; 4] {
    let mut hits = [false; 4];
    for obstacle in obstacles {
        let index = class_index(obstacle.class);
        for disk in &obstacle.bodies {
            let limit = ROBOT_RADIUS_MM.saturating_add(disk.radius.get());
            if segment_within(from, to, disk.center, limit) {
                hits[index] = true;
            }
        }
    }
    hits
}

fn extend(points: &mut Vec<Point>, goal: Point) {
    for _ in 0..20_000 {
        let Some(from) = points.last().copied() else {
            return;
        };
        if from == goal {
            return;
        }
        let next = toward(from, goal);
        if next == from {
            return;
        }
        points.push(next);
    }
}

fn toward(from: Point, to: Point) -> Point {
    let dx = to.x().get().saturating_sub(from.x().get());
    let dy = to.y().get().saturating_sub(from.y().get());
    let (sx, sy) = if dx == 0 && dy == 0 {
        (0, 0)
    } else if dx.saturating_abs() >= dy.saturating_abs() {
        (dx.clamp(-STEP_MM, STEP_MM), 0)
    } else {
        (0, dy.clamp(-STEP_MM, STEP_MM))
    };
    point(
        from.x().get().saturating_add(sx),
        from.y().get().saturating_add(sy),
    )
}

fn ceil_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let dx = i128::from(bx) - i128::from(ax);
    let dy = i128::from(by) - i128::from(ay);
    let squared = dx * dx + dy * dy;
    if squared <= 0 {
        return 0;
    }
    let squared = u128::try_from(squared).unwrap_or(u128::MAX);
    let root = isqrt(squared);
    let len = if root.saturating_mul(root) == squared {
        root
    } else {
        root.saturating_add(1)
    };
    i32::try_from(len).unwrap_or(i32::MAX)
}

fn isqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = 1u128 << ((127 - n.leading_zeros()) / 2 + 1);
    loop {
        let next = (x + n / x) >> 1;
        if next >= x {
            return x;
        }
        x = next;
    }
}

fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "idle",
        Phase::AwaitPoint => "await",
        Phase::Naming => "naming",
        Phase::Picking => "picking",
        Phase::Signaling => "signaling",
        Phase::Following => "following",
        Phase::Lost => "lost",
        Phase::Releasing => "releasing",
        Phase::HandoffFailed => "handoff-failed",
        Phase::WrongBag => "wrong-bag",
        Phase::PickFailed => "pick-failed",
        Phase::Dropped => "dropped",
        Phase::Complete => "complete",
        Phase::TimeUp => "time-up",
    }
}

fn custody_name(observed: &domain::Observation, bag: Id) -> &'static str {
    match observed
        .bags
        .iter()
        .find(|item| item.id == bag)
        .map(|item| item.custody)
    {
        Some(Custody::Operator) => "operator",
        Some(Custody::Gripper) => "gripper",
        Some(Custody::Free) => "free",
        None => "missing",
    }
}

fn feed_name(error: FeedError) -> &'static str {
    match error {
        FeedError::TimeUp => "time-up",
        FeedError::Split => "split",
        FeedError::Terminal => "terminal",
        FeedError::Arm(error) => match error {
            manipulation::GripError::Blocked => "blocked",
            manipulation::GripError::Empty => "empty",
            manipulation::GripError::OutOfReach => "reach",
            _ => "arm",
        },
        FeedError::Trip(_) => "trip",
        FeedError::NotAReport => "report",
        FeedError::NotACommand => "command",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use task_3_1::{
        Award, BAR, CROWD, DROP, FOLLOW, Finding, FindingResult, HARD, HELP_CAP, Help, PICK, SEE,
        SMALL, TOTAL, absent_choice, alternative_start, apply_finding, bonus_line, drop_shares_cap,
        fair_play_list, finding_from_contact, finding_from_low_score, finding_from_trajectory,
        first_drop, follow_after_recovery, follow_withheld_without_return, help_total,
        interpret_start_signal, network_blocks_score, outstanding_is_automatic,
        partial_without_the_car, pick_line, positive_sum, second_drop, social_bonus_added,
    };

    fn main_at(seed: u64, car: Option<Point>) -> EpisodeReport {
        run(Script::Main {
            seed,
            car,
            then_return: false,
            far_bag: false,
        })
    }

    fn near(a: (i32, i32), b: (i32, i32)) -> bool {
        let dx = i64::from(a.0) - i64::from(b.0);
        let dy = i64::from(a.1) - i64::from(b.1);
        dx * dx + dy * dy <= i64::from(ARRIVE_MM) * i64::from(ARRIVE_MM)
    }

    #[test]
    fn seeded_main_reaches_the_car() {
        let report = main_at(1, None);
        assert!(report.main_passed, "CML-TASK {}", report.summary());
        assert_eq!(report.operator, report.car, "CML-TASK");
        assert!(near(report.robot, report.operator), "CML-TASK");
        assert_eq!(report.custody, "operator", "CML-TASK");
        assert!(report.trip_flag);
        assert!(report.local_cue, "CML-SET-CAR");
        assert!(report.car_hidden, "CML-SET-CAR");
        assert!(report.followed_operator, "CML-SCORE-FOLLOW");
        assert!(report.follow_moves > 0, "CML-SCORE-FOLLOW");
        assert!(report.legal_motion, "CML-SET-CAR");
        assert!(report.longest_step_mm <= STEP_MM, "CML-SET-CAR");
        assert!(report.held_tracks, "CML-SCORE-PICK");
        let early = run(Script::EarlyCue { seed: 1 });
        assert!(early.trip_flag, "CML-TASK");
        assert!(!early.main_passed, "CML-TASK");
        assert_ne!(early.operator, early.car, "CML-SET-CAR");
        let jump = run(Script::Jump { seed: 1 });
        assert!(jump.trip_flag, "CML-SET-CAR");
        assert!(!jump.main_passed, "CML-SET-CAR");
        assert!(jump.longest_step_mm > STEP_MM, "CML-SET-CAR");
        let short = run(Script::StopShort { seed: 1 });
        assert!(short.trip_flag, "CML-SET-CAR");
        assert!(!short.main_passed, "CML-SET-CAR");
        assert_ne!(short.operator, short.car, "CML-SET-CAR");
        let unseen = run(Script::Unseen { seed: 1 });
        assert!(!unseen.main_passed, "CML-SET-CAR");
        assert!(!unseen.local_cue, "CML-SET-CAR");
        assert!(!unseen.trip_flag, "CML-SET-CAR");
        let reversed = run(Script::Reversed { seed: 1 });
        assert!(reversed.trip_flag, "CML-TASK");
        assert!(!reversed.main_passed, "CML-TASK");
        assert_eq!(reversed.operator, (0, 1_000), "CML-TASK");
        assert_ne!(reversed.operator, reversed.car, "CML-GEN-PRIORITY");
        assert!(report.start_inside, "CML-SET-PLACE");
        assert!(report.start_fixed, "CML-SET-PLACE");
        assert_eq!(
            report
                .trace
                .first()
                .map(|sample| (sample.robot_x, sample.robot_y)),
            Some((0, 0)),
            "CML-SET-PLACE"
        );
        assert!(report.bags_ok, "CML-SET-BAGS");
        assert!(!run(Script::OneBag { seed: 1 }).main_passed, "CML-SET-BAGS");
        assert!(report.bag_kind_ok, "CML-BAG-DEF");
        assert_eq!(PICK, 100, "CML-BAG-DEF");
        assert!(
            report.avoided == [true, true, true, true],
            "CML-SET-OBST {}",
            report.summary()
        );
        assert!(report.car.0 > 2_500, "CML-SET-OBST");
        assert!(
            report.trace.iter().any(|sample| sample.robot_x > 2_500),
            "CML-SET-OBST"
        );
        assert!(report.pickup_before_follow, "CML-PROC-ORDER");
        assert!(report.handoff_at_car, "CML-PROC-ORDER");
        assert!(!report.returned, "CML-PROC-OPT");
        assert!(!follow_withheld_without_return(), "CML-PROC-OPT");
        assert_eq!(report.shown_bag, Some(report.pointed), "CML-SCORE-SEE");
        assert_eq!(SEE, 15, "CML-SCORE-SEE");
        assert!(report.grasped_pointed, "CML-SCORE-PICK");
        assert_eq!(PICK, 100, "CML-SCORE-PICK");
        assert!(
            !run(Script::WrongBag { seed: 1 }).main_passed,
            "CML-SCORE-PICK"
        );
        assert_eq!(pick_line(false, true), 0, "CML-GEN-LIST");
        assert_eq!(FOLLOW, 300, "CML-SCORE-FOLLOW");
        assert!(report.avoided[2], "CML-SCORE-CROWD");
        assert_eq!(CROWD, 50, "CML-SCORE-CROWD");
        assert!(report.avoided[0], "CML-SCORE-SMALL");
        assert_eq!(SMALL, 50, "CML-SCORE-SMALL");
        assert!(report.avoided[1], "CML-SCORE-HARD");
        assert_eq!(HARD, 50, "CML-SCORE-HARD");
        assert!(report.avoided[3], "CML-SCORE-BAR");
        assert_eq!(BAR, 50, "CML-SCORE-BAR");
        assert!(
            report.summary().contains("assumption clearance-mm"),
            "CML-ASM-AVOID"
        );
        assert!(
            !report.summary().contains("printed clearance"),
            "CML-ASM-AVOID"
        );
        assert!(report.summary().contains("assumption size"), "CML-ASM-SIZE");
        assert!(
            report.summary().contains("order-not-required"),
            "CML-ASM-ORDER"
        );
        assert!(
            report.summary().contains("public-place-not-certified"),
            "CML-ASM-SPACE"
        );
        assert!(
            report.summary().contains("hardware CML-HW-OUT not-run"),
            "CML-ASM-SPACE"
        );
        assert!(
            report.car_outside && report.start_inside,
            "CML-GEN-PRIORITY"
        );
        let stranger = run(Script::WrongPerson { seed: 1 });
        assert!(!stranger.main_passed, "CML-SCORE-FOLLOW");
        assert_eq!(stranger.fail, Some("wrong-person"), "CML-SCORE-FOLLOW");
        let disk = run(Script::BlockedHandoff { seed: 1 });
        assert!(!disk.main_passed, "CML-PROC-ORDER");
        assert_eq!(disk.phase, "releasing", "CML-PROC-ORDER");
        assert_eq!(disk.custody, "gripper", "CML-PROC-ORDER");
    }

    #[test]
    fn varied_cars_are_not_a_hardcoded_point() {
        let mut orders = Vec::new();
        let mut crowds = Vec::new();
        for seed in 0..8 {
            let report = main_at(seed, None);
            assert!(
                report.main_passed,
                "CML-SET-CAR seed {seed} {}",
                report.summary()
            );
            assert_eq!(report.operator, (8_000, -1_500), "CML-SET-CAR");
            assert_ne!(report.robot, report.car, "CML-SET-CAR");
            orders.push(report.order);
            crowds.push(report.crowd);
            assert!((3..=4).contains(&report.crowd), "CML-SET-CROWD");
            assert_eq!(
                report.crowd,
                3 + usize::try_from(seed % 2).unwrap_or(0),
                "CML-SET-CROWD"
            );
            assert!(report.trace_ok && report.clocks_match, "CML-TIME");
        }
        assert!(
            orders.windows(2).any(|pair| pair[0] != pair[1]),
            "CML-ASM-ORDER"
        );
        assert!(crowds.contains(&3) && crowds.contains(&4), "CML-SET-CROWD");
        let cars = [
            (2_600, 2_000),
            (7_000, -2_000),
            (9_000, 3_500),
            (12_000, -500),
            (8_000, 1_600),
        ];
        let mut robots = Vec::new();
        for (index, (x, y)) in cars.iter().enumerate() {
            let report = main_at(u64::try_from(index).unwrap_or(0), Some(point(*x, *y)));
            assert!(
                report.main_passed,
                "CML-SET-CAR {x},{y} {}",
                report.summary()
            );
            assert_eq!(report.operator, (*x, *y), "CML-SET-CAR");
            assert!(report.car_hidden, "CML-SET-CAR");
            robots.push(report.robot);
        }
        assert!(
            robots.windows(2).any(|pair| pair[0] != pair[1]),
            "CML-SET-CAR"
        );
        let wide = main_at(u64::MAX, Some(point(7_500, 0)));
        assert!(wide.main_passed, "CML-SET-CAR {}", wide.summary());
        let mut state = 0x1234_5678_u64;
        for _ in 0..6 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let x = 6_800 + i32::try_from(state % 4_000).unwrap_or(0);
            let y = -4_000 + i32::try_from((state >> 8) % 8_000).unwrap_or(0);
            let report = main_at(state, Some(point(x, y)));
            assert!(
                report.main_passed,
                "CML-SET-CAR {x},{y} {}",
                report.summary()
            );
            assert_eq!(report.operator, (x, y), "CML-SET-CAR");
        }
    }

    #[test]
    fn optional_return_joins_the_same_crowd() {
        let outbound = main_at(1, None);
        assert!(outbound.main_passed, "CML-PROC-OPT");
        assert!(!outbound.returned, "CML-PROC-OPT");
        assert_eq!(FOLLOW, 300, "CML-PROC-OPT");
        let back = run(Script::Main {
            seed: 1,
            car: None,
            then_return: true,
            far_bag: false,
        });
        assert!(back.main_passed, "CML-SCORE-BACK {}", back.summary());
        assert!(back.returned, "CML-SCORE-BACK");
        assert!(back.queued, "CML-SCORE-QUEUE");
        assert!(back.queue_same, "CML-SET-CROWD");
        assert!(back.robot.0 <= 2_500, "CML-SCORE-BACK");
        assert_eq!(back.operator, back.car, "CML-SCORE-BACK");
        assert_eq!(back.custody, "operator", "CML-SCORE-BACK");
        assert_eq!(BACK, 100, "CML-SCORE-BACK");
        assert_eq!(back.bonus_back, 100, "CML-SCORE-BACK");
        assert_eq!(task_3_1::QUEUE, 300, "CML-SCORE-QUEUE");
        assert_eq!(back.bonus_queue, 300, "CML-SCORE-QUEUE");
        assert!((3..=4).contains(&back.crowd), "CML-ASM-FEW");
        assert!(
            back.summary().contains("assumption few-return"),
            "CML-ASM-FEW"
        );
        let other = run(Script::Main {
            seed: 0,
            car: None,
            then_return: true,
            far_bag: false,
        });
        assert!(
            other.queued && other.crowd != back.crowd,
            "CML-SET-CROWD {}",
            other.summary()
        );
        let help = run(Script::HelpBonus { seed: 1 });
        assert!(help.returned, "CML-GEN-BONUS {}", help.summary());
        assert_eq!(help.bonus_back, 0, "CML-GEN-BONUS");
        assert_eq!(help.bonus_queue, 0, "CML-GEN-BONUS");
        assert_eq!(bonus_line(BACK, true), 0, "CML-GEN-BONUS");
        let swapped = run(Script::DifferentQueue { seed: 1 });
        assert!(swapped.main_passed, "CML-SET-CROWD");
        assert!(!swapped.queue_same, "CML-SET-CROWD");
        assert!(!swapped.queued, "CML-SCORE-QUEUE");
    }

    #[test]
    fn one_clock_covers_the_episode() {
        let report = main_at(1, None);
        assert!(report.clocks_match, "CML-TIME {}", report.summary());
        assert!(report.trace_ok, "CML-TIME");
        assert!(report.clock_ms <= LIMIT_MS, "CML-TIME");
        assert_eq!(LIMIT_MS, 300_000, "CML-TIME");
        let mut previous = 0u64;
        for sample in &report.trace {
            assert!(sample.ms >= previous, "CML-TIME");
            previous = sample.ms;
        }
        for pair in report.trace.windows(2) {
            let dx = i64::from(pair[1].robot_x) - i64::from(pair[0].robot_x);
            let dy = i64::from(pair[1].robot_y) - i64::from(pair[0].robot_y);
            assert!(
                dx * dx + dy * dy <= i64::from(STEP_MM) * i64::from(STEP_MM),
                "CML-TIME"
            );
        }
        assert!(
            report.summary().contains("assumption clock-start begin"),
            "CML-ASM-CLOCK"
        );
        assert!(
            !report.summary().contains("printed start signal"),
            "CML-ASM-CLOCK"
        );
        let budget = run(Script::Budget { seed: 1 });
        assert!(budget.moved_on_limit, "CML-TIME {}", budget.summary());
        assert!(budget.frozen_after_limit, "CML-TIME");
        assert_eq!(budget.clock_ms, LIMIT_MS + 1, "CML-TIME");
        assert_eq!(budget.phase, "time-up", "CML-TIME");
        assert!(!budget.main_passed, "CML-TIME");
        assert!(!budget.trip_flag, "CML-TIME");
        let stuck = run(Script::Stuck { seed: 1 });
        assert_eq!(stuck.fail, Some("stuck"), "CML-TIME");
        assert!(stuck.clock_ms > 1_000, "CML-TIME");
        assert_eq!(stuck.robot, (0, 0), "CML-TIME");
        assert!(!stuck.main_passed, "CML-TIME");
        let overflow = run(Script::Overflow);
        assert_eq!(
            overflow.fail,
            Some("overflow"),
            "CML-TIME {}",
            overflow.summary()
        );
        assert_eq!(overflow.clock_ms, u64::MAX - 40, "CML-TIME");
        assert_eq!(overflow.phase, "await", "CML-TIME");
        assert!(overflow.clocks_match, "CML-TIME");
        assert!(!overflow.main_passed, "CML-TIME");
        let cancel = run(Script::Cancel { seed: 1 });
        assert!(cancel.cancelled, "CML-TIME");
        assert!(!cancel.main_passed, "CML-TIME");
        assert_ne!(cancel.operator, cancel.car, "CML-SET-CAR");
        let loss = run(Script::Loss { seed: 1 });
        assert!(loss.main_passed, "CML-OPEN-FOLLOW {}", loss.summary());
        assert_eq!(follow_after_recovery(), Award::Undecided, "CML-OPEN-FOLLOW");
        let far = run(Script::Main {
            seed: 2,
            car: None,
            then_return: false,
            far_bag: true,
        });
        assert!(far.approached, "CML-SCORE-PICK {}", far.summary());
        assert!(far.main_passed, "CML-SCORE-PICK");
        assert!(far.clocks_match, "CML-ASM-CLOCK");
    }

    #[test]
    fn printed_sheet_stays_undecided_on_open_rows() {
        assert_eq!(positive_sum(), TOTAL, "CML-SCORE-TOTAL");
        assert_eq!(TOTAL, 1015, "CML-SCORE-TOTAL");
        assert_eq!(task_3_1::OUTSTANDING, 101, "CML-SCORE-OUT");
        assert!(!outstanding_is_automatic(), "CML-SCORE-OUT");
        assert_eq!(first_drop(), DROP, "CML-PEN-DROP");
        assert_eq!(DROP, -50, "CML-PEN-DROP");
        assert_eq!(positive_sum(), 1015, "CML-PEN-DROP");
        let dropped = run(Script::Drop { seed: 1 });
        assert_eq!(dropped.fail, Some("dropped"), "CML-PEN-DROP");
        assert_eq!(dropped.custody, "free", "CML-PEN-DROP");
        assert!(!dropped.main_passed, "CML-PEN-DROP");
        assert_eq!(second_drop(), Award::Undecided, "CML-OPEN-DROP");
        assert_eq!(
            help_total(&[(Help::Natural, true); 6]),
            -HELP_CAP,
            "CML-DER-CAP"
        );
        assert_ne!(
            help_total(&[(Help::Natural, true); 6]),
            6 * -50,
            "CML-DER-CAP"
        );
        assert_eq!(help_total(&[(Help::Natural, false); 4]), 0, "CML-PEN-IF");
        assert_eq!(task_3_1::help_points(Help::Natural), -50, "CML-PEN-NAT");
        assert_eq!(
            task_3_1::help_points(Help::Unnatural),
            -100,
            "CML-PEN-UNNAT"
        );
        assert_eq!(task_3_1::help_points(Help::ComeBack), -150, "CML-PEN-BACK");
        assert_eq!(task_3_1::help_points(Help::Contact), -200, "CML-PEN-TOUCH");
        assert_eq!(drop_shares_cap(), Award::Undecided, "CML-OPEN-CAP");
        assert_eq!(partial_without_the_car(), Award::Undecided, "CML-GEN-MAIN");
        assert_eq!(
            partial_without_the_car(),
            Award::Undecided,
            "CML-OPEN-PARTIAL"
        );
        assert_eq!(alternative_start(), Award::Undecided, "CML-SHEET-START");
        assert_eq!(
            interpret_start_signal("door"),
            Award::Undecided,
            "CML-SHEET-START"
        );
        assert_eq!(task_3_1::ABSENT_PROSE, 250, "CML-PROSE-ABSENT");
        assert_eq!(task_3_1::ABSENT_NOTICE_MIN, 60, "CML-PROSE-ABSENT");
        assert_eq!(task_3_1::ABSENT_SHEET, -500, "CML-SHEET-ABSENT");
        assert_eq!(absent_choice(), Award::Undecided, "CML-OPEN-ABSENT");
        assert_eq!(follow_after_recovery(), Award::Undecided, "CML-OPEN-FOLLOW");
        assert!(fair_play_list().is_none(), "CML-GEN-CHEAT");
        assert_eq!(
            apply_finding(Finding::Cheat),
            FindingResult::Disqualified { points: -500 },
            "CML-GEN-CHEAT"
        );
        assert!(finding_from_low_score(0).is_none(), "CML-GEN-FAKE");
        assert_eq!(
            apply_finding(Finding::NotTrying),
            FindingResult::Penalty(-250),
            "CML-GEN-FAKE"
        );
        assert!(finding_from_contact(true).is_none(), "CML-GEN-HIT");
        assert_eq!(
            apply_finding(Finding::MajorCollision),
            FindingResult::ScoreZero,
            "CML-GEN-HIT"
        );
        assert!(finding_from_trajectory().is_none(), "CML-GEN-JURY");
        assert_eq!(
            apply_finding(Finding::MissingJury),
            FindingResult::Penalty(-250),
            "CML-GEN-JURY"
        );
        assert!(!network_blocks_score(true), "CML-GEN-NET");
        assert_eq!(social_bonus_added(), 0, "CML-GEN-SOCIAL");
        assert_eq!(pick_line(true, false), 100, "CML-GEN-LIST");
        let products = [1015 * 10 / 100, 1015 * 20 / 100, 1015 * 30 / 100];
        assert!(products.contains(&101), "CML-SCORE-OUT");
        assert!(
            !products.contains(&203)
                || task_3_1::positive_lines()
                    .iter()
                    .all(|(_, points)| *points != 203),
            "CML-GEN-PCT"
        );
        assert!(
            task_3_1::positive_lines()
                .iter()
                .all(|(_, points)| *points != 203 && *points != 304),
            "CML-GEN-PCT"
        );
        let report = main_at(1, None);
        for id in task_3_1::OPEN {
            assert!(
                report.summary().contains(&format!("open {id} undecided")),
                "{id}"
            );
            assert!(!report.summary().contains(&format!("{id} pass")), "{id}");
        }
    }

    #[test]
    fn hardware_rows_stay_not_run() {
        let report = run(Script::OneBag { seed: 4 });
        assert!(!report.main_passed, "CML-HW-OUT");
        for id in ["CML-HW-OUT", "CML-HW-HIT", "CML-HW-SPL"] {
            assert!(
                report
                    .summary()
                    .contains(&format!("hardware {id} not-run no-device")),
                "{id}"
            );
            assert!(!report.summary().contains(&format!("{id} pass")), "{id}");
        }
    }
}
