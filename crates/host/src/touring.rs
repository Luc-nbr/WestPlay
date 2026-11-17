//! Dashly's touring-coach company: a coach-travel firm of the player's own (its name theirs, no real firm's
//! look), run locally alongside the truck features. Day trips, multi-day tours across Europe, school trips and
//! charters, generated from the places on the shown map (its cities, hotels, parkings and fuel stations); seats
//! booked by simulated passengers (reputation, price, season, how far ahead); each trip driven as phases
//! (to the first pick-up point, boarding, outbound with coffee and meal stops, the stay with excursions, the
//! return, passengers off, back to the depot), stops reached by standing still inside their radius.
//!
//! Passengers' mood comes from being on time at the pick-up points, the comfort of the ride (harsh braking,
//! acceleration and cornering in the telemetry), the breaks taken (the tachograph's own 4.5-hour rule, and the
//! coffee stops' length) and announcements made over the coach's PA (a bonus). Money: seats sold, less fuel
//! (the telemetry's tank), tolls and ferries (estimated: the plugin has no gameplay events yet), hotels and the
//! driver's wage. Reputation follows the mood and feeds future bookings.
//!
//! Times are the game's clock (game minutes, as the telemetry's `game_time`); places are the game's world
//! (metres: x east, z south). Everything is kept in `touring/book.json` in Dashly's data folder, as the
//! tachograph keeps its book.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// What the companion and the cab ask of the company, for the host to carry out on its next tick.
pub static ACTIONS: Mutex<Vec<Action>> = Mutex::new(Vec::new());

/// Asks for `action` (from any thread).
pub fn request(action: Action) {
    if let Ok(mut actions) = ACTIONS.lock() {
        actions.push(action);
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum Action {
    /// Take a trip from the catalogue: it becomes the active trip.
    Start { id: u64 },
    /// Call the active trip off (refunds; reputation suffers).
    Cancel,
    /// A new catalogue (the trips without bookings replaced).
    Refresh,
    /// Record a point "here" (where the vehicle stands).
    Record { kind: StopKind, name: String },
    /// Forget a recorded point.
    Forget { index: usize },
    /// Skip the next stop (it can't be reached): counted as missed.
    Skip,
    /// Start again from nothing: balance, reputation, history.
    Reset,
}

pub const MINUTES_PER_DAY: f64 = 1440.0;
/// The map's scale between towns (ETS2's 19): world metres to the game's kilometres.
const SCALE: f64 = 19.0;
/// Roads aren't straight: the way is this much longer than the straight line.
const ROAD_FACTOR: f64 = 1.3;
/// The coach's average speed, game km/h (towns, motorways).
const AVERAGE_KMH: f64 = 75.0;
/// Coffee stops at most this far apart (game minutes of driving): well inside the 4.5 hours.
const COFFEE_EVERY: f64 = 150.0;
/// Driving in a day before an overnight stop on the way (game minutes).
const DAY_DRIVING: f64 = 540.0;
/// A stop counts once the vehicle has stood inside its radius this long.
const STAND: Duration = Duration::from_secs(4);
/// Standing still: slower than this (m/s).
const STILL: f32 = 1.0;
/// Harsh: longitudinal deceleration, acceleration and lateral acceleration (m/s²).
const HARSH_BRAKE: f32 = 3.2;
const HARSH_ACCELERATION: f32 = 2.4;
const HARSH_CORNER: f32 = 3.0;
/// One harsh moment counted once, however long it lasts.
const HARSH_GAP: Duration = Duration::from_secs(3);
/// Costs.
const FUEL_PER_LITRE: f64 = 1.65;
const TOLL_PER_KM: f64 = 0.09;
const FERRY: f64 = 380.0;
const HOTEL_PER_NIGHT: f64 = 48.0;
const WAGE_PER_DAY: f64 = 230.0;
/// How many trips the catalogue offers.
const CATALOGUE: usize = 8;
const KEEP_HISTORY: usize = 60;

/// What a stop is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopKind {
    Depot,
    Pickup,
    Coffee,
    Meal,
    Destination,
    Hotel,
    Excursion,
    Dropoff,
}

impl StopKind {
    /// How close counts as there (metres).
    fn radius(self) -> f64 {
        match self {
            StopKind::Depot => 250.0,
            StopKind::Pickup | StopKind::Dropoff => 150.0,
            StopKind::Coffee | StopKind::Meal => 180.0,
            StopKind::Hotel => 250.0,
            StopKind::Destination | StopKind::Excursion => 700.0,
        }
    }

    /// How long the stop is planned to last (game minutes).
    fn dwell(self) -> f64 {
        match self {
            StopKind::Pickup | StopKind::Dropoff => 10.0,
            StopKind::Coffee => 25.0,
            StopKind::Meal => 50.0,
            _ => 0.0,
        }
    }
}

/// A place: a name, where it is in the game's world, how close counts, and its map node (for the game's GPS).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub name: String,
    pub x: f64,
    pub z: f64,
    pub radius: f64,
    /// The map node, hex (`None`: the nearest is looked up).
    #[serde(default)]
    pub node: Option<String>,
}

impl Point {
    fn distance(&self, x: f64, z: f64) -> f64 {
        (self.x - x).hypot(self.z - z)
    }
}

/// A point the player recorded "here".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recorded {
    pub kind: StopKind,
    pub point: Point,
}

/// Which part of the trip a stop belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Leg {
    Boarding,
    Outbound,
    Stay,
    Return,
    Unloading,
    ToDepot,
}

/// Where the trip is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// From the depot to the first pick-up point.
    ToFirstPickup,
    Boarding,
    Outbound,
    Stay,
    Return,
    Unloading,
    ToDepot,
    Done,
}

/// One stop of a trip: where, what, when planned (game minutes), on which day, how many board or leave, and when
/// it was reached and left.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stop {
    pub kind: StopKind,
    pub leg: Leg,
    pub point: Point,
    pub at: f64,
    pub day: u32,
    #[serde(default)]
    pub passengers: u32,
    #[serde(default)]
    pub reached: Option<f64>,
    #[serde(default)]
    pub left: Option<f64>,
    #[serde(default)]
    pub missed: bool,
}

impl Stop {
    fn new(kind: StopKind, leg: Leg, point: Point, at: f64, day: u32) -> Self {
        Self { kind, leg, point, at, day, passengers: 0, reached: None, left: None, missed: false }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TripKind {
    DayTrip,
    Tour,
    School,
    Charter,
}

/// A trip in the catalogue (or driven): what, where to, how long, when it leaves, the seat's price (a charter's
/// or a school's: the whole contract), seats and bookings, its stops.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trip {
    pub id: u64,
    pub kind: TripKind,
    /// What the catalogue calls it (a tour's region, else the city), and the city it goes to.
    pub title: String,
    pub destination: String,
    pub days: u32,
    pub departure: f64,
    /// When it was put in the catalogue (game minutes): bookings come in from then.
    pub listed: f64,
    pub price: f64,
    pub seats: u32,
    pub booked: u32,
    /// Game km, the whole way, as planned.
    pub distance: f64,
    pub stops: Vec<Stop>,
}

impl Trip {
    /// What it earns: seats sold, or the contract.
    pub fn income(&self) -> f64 {
        match self.kind {
            TripKind::Charter | TripKind::School => self.price,
            _ => self.price * f64::from(self.booked),
        }
    }

    fn nights(&self) -> u32 {
        self.days.saturating_sub(1)
    }
}

/// What the passengers felt, as counted along the way.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Score {
    /// Minutes late at each pick-up point reached (negative: early).
    pub pickups_late: Vec<f64>,
    pub harsh_brakes: u32,
    pub harsh_accelerations: u32,
    pub harsh_corners: u32,
    /// Breaks the tachograph counted (45, or 15 + 30) with passengers on board.
    pub breaks: u32,
    /// Times the 4.5 hours were driven past with passengers on board.
    pub overruns: u32,
    /// Coffee and meal stops long enough, and too short.
    pub good_stops: u32,
    pub short_stops: u32,
    pub missed_stops: u32,
    pub announcements: u32,
}

/// The mood (0 to 100) from what was counted, and its parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mood {
    pub punctuality: f64,
    pub comfort: f64,
    pub breaks: f64,
    pub bonus: f64,
    pub total: f64,
}

pub fn mood(score: &Score) -> Mood {
    let punctuality = if score.pickups_late.is_empty() {
        100.0
    } else {
        let each = |late: f64| (100.0 - (late - 3.0).max(0.0) * 4.0).clamp(0.0, 100.0);
        score.pickups_late.iter().copied().map(each).sum::<f64>() / score.pickups_late.len() as f64
    };
    let comfort = (100.0
        - 4.0 * f64::from(score.harsh_brakes)
        - 2.5 * f64::from(score.harsh_accelerations)
        - 3.0 * f64::from(score.harsh_corners))
    .clamp(0.0, 100.0);
    let breaks = (80.0 + 8.0 * f64::from(score.good_stops + score.breaks)
        - 10.0 * f64::from(score.short_stops)
        - 20.0 * f64::from(score.overruns)
        - 12.0 * f64::from(score.missed_stops))
    .clamp(0.0, 100.0);
    let bonus = (2.5 * f64::from(score.announcements)).min(10.0);
    let total = (0.35 * punctuality + 0.35 * comfort + 0.30 * breaks + bonus).clamp(0.0, 100.0);
    Mood { punctuality, comfort, breaks, bonus, total }
}

/// Reputation's step after a trip with this mood.
pub fn reputation_step(mood: f64) -> f64 {
    ((mood - 62.0) / 8.0).clamp(-6.0, 4.0)
}

/// What a trip cost.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Costs {
    pub fuel: f64,
    pub tolls: f64,
    pub ferries: f64,
    pub hotel: f64,
    pub wage: f64,
}

impl Costs {
    pub fn total(&self) -> f64 {
        self.fuel + self.tolls + self.ferries + self.hotel + self.wage
    }
}

/// What was counted while driving: litres used, game km driven, ferries (or trains) taken.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Meter {
    pub litres: f64,
    pub km: f64,
    pub ferries: u32,
}

/// The bill: fuel used, tolls by the km, ferries, the hotel's nights for the passengers and the driver, the
/// driver's days.
pub fn costs(trip: &Trip, meter: &Meter) -> Costs {
    let people = f64::from(trip.booked + 1);
    Costs {
        fuel: meter.litres * FUEL_PER_LITRE,
        tolls: meter.km * TOLL_PER_KM,
        ferries: f64::from(meter.ferries) * FERRY,
        hotel: f64::from(trip.nights()) * people * HOTEL_PER_NIGHT,
        wage: f64::from(trip.days.max(1)) * WAGE_PER_DAY,
    }
}

/// A trip driven, as the history keeps it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finished {
    pub id: u64,
    pub kind: TripKind,
    pub destination: String,
    pub days: u32,
    pub departure: f64,
    pub ended: f64,
    pub passengers: u32,
    pub income: f64,
    pub costs: Costs,
    pub profit: f64,
    pub mood: f64,
    pub reputation: f64,
    pub cancelled: bool,
}

/// The trip being driven: the trip, which stop is next, who's on board, what was counted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Active {
    pub trip: Trip,
    pub next: usize,
    pub on_board: u32,
    pub score: Score,
    pub meter: Meter,
    /// Standing at a coffee or meal stop since (game minutes): its length counted as the coach leaves.
    #[serde(default)]
    pub resting_at: Option<usize>,
    /// The announcement asked for now, and the ones already asked (never twice).
    #[serde(default)]
    pub prompt: Option<Prompt>,
    #[serde(default)]
    pub prompted: Vec<String>,
}

impl Active {
    pub fn new(trip: Trip) -> Self {
        Self {
            trip,
            next: 0,
            on_board: 0,
            score: Score::default(),
            meter: Meter::default(),
            resting_at: None,
            prompt: None,
            prompted: Vec::new(),
        }
    }

    pub fn next_stop(&self) -> Option<&Stop> {
        self.trip.stops.get(self.next)
    }

    pub fn phase(&self) -> Phase {
        match self.next_stop() {
            None => Phase::Done,
            Some(stop) => match stop.leg {
                Leg::Boarding if self.on_board == 0 => Phase::ToFirstPickup,
                Leg::Boarding => Phase::Boarding,
                Leg::Outbound => Phase::Outbound,
                Leg::Stay => Phase::Stay,
                Leg::Return => Phase::Return,
                Leg::Unloading => Phase::Unloading,
                Leg::ToDepot => Phase::ToDepot,
            },
        }
    }

    /// Passengers still to collect.
    pub fn to_collect(&self) -> u32 {
        self.trip.stops[self.next.min(self.trip.stops.len())..]
            .iter()
            .filter(|stop| stop.kind == StopKind::Pickup)
            .map(|stop| stop.passengers)
            .sum()
    }

    /// The next stop reached at `now` (game minutes): passengers on or off, punctuality counted, the next one.
    /// Returns the stop reached.
    pub fn reach(&mut self, now: f64) -> Option<Stop> {
        let index = self.next;
        let stop = self.trip.stops.get_mut(index)?;
        stop.reached = Some(now);
        match stop.kind {
            StopKind::Pickup => {
                self.on_board += stop.passengers;
                self.score.pickups_late.push(now - stop.at);
            }
            StopKind::Dropoff => self.on_board = self.on_board.saturating_sub(stop.passengers),
            StopKind::Coffee | StopKind::Meal => self.resting_at = Some(index),
            _ => {}
        }
        let reached = stop.clone();
        self.next += 1;
        Some(reached)
    }

    /// The next stop skipped (not reachable): counted as missed; pick-ups' passengers come on anyway (they took
    /// a taxi to the next), drop-offs' get off.
    pub fn skip(&mut self) {
        let Some(stop) = self.trip.stops.get_mut(self.next) else { return };
        stop.missed = true;
        match stop.kind {
            StopKind::Pickup => self.on_board += stop.passengers,
            StopKind::Dropoff => self.on_board = self.on_board.saturating_sub(stop.passengers),
            _ => {}
        }
        if stop.kind != StopKind::Depot {
            self.score.missed_stops += 1;
        }
        self.next += 1;
    }

    /// The coach left the coffee or meal stop it stood at, at `now`: its length counted.
    pub fn leave_rest(&mut self, now: f64) {
        let Some(index) = self.resting_at.take() else { return };
        let stop = &mut self.trip.stops[index];
        stop.left = Some(now);
        let minutes = now - stop.reached.unwrap_or(now);
        let enough = if stop.kind == StopKind::Meal { 30.0 } else { 15.0 };
        if minutes >= enough {
            self.score.good_stops += 1;
        } else {
            self.score.short_stops += 1;
        }
    }
}

/// Whether the vehicle stands at a point: inside its radius and still, for [`STAND`].
#[derive(Debug, Default)]
pub struct Arrival {
    since: Option<Instant>,
}

impl Arrival {
    pub fn update(&mut self, point: &Point, x: f64, z: f64, speed: f32, now: Instant) -> bool {
        if point.distance(x, z) <= point.radius && speed.abs() < STILL {
            let since = *self.since.get_or_insert(now);
            now.duration_since(since) >= STAND
        } else {
            self.since = None;
            false
        }
    }

    pub fn reset(&mut self) {
        self.since = None;
    }
}

/// Harsh moments, from the telemetry's speed and heading as they change.
#[derive(Debug, Default)]
pub struct Comfort {
    last: Option<(f32, f32, Instant)>,
    brake_at: Option<Instant>,
    accelerate_at: Option<Instant>,
    corner_at: Option<Instant>,
}

/// A harsh moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Harsh {
    Brake,
    Accelerate,
    Corner,
}

impl Comfort {
    /// The speed (m/s) and heading (turns) at `now`; a harsh moment, once each.
    pub fn update(&mut self, speed: f32, heading: f32, now: Instant) -> Option<Harsh> {
        let (speed_before, heading_before, before) = self.last.replace((speed, heading, now))?;
        let dt = now.duration_since(before).as_secs_f32();
        if !(0.02..=0.5).contains(&dt) {
            return None;
        }
        let acceleration = (speed.abs() - speed_before.abs()) / dt;
        let mut turn = heading - heading_before;
        turn -= turn.round();
        let lateral = (turn * std::f32::consts::TAU / dt * speed).abs();
        let once = |at: &mut Option<Instant>| {
            let fresh = at.is_none_or(|at| now.duration_since(at) >= HARSH_GAP);
            if fresh {
                *at = Some(now);
            }
            fresh
        };
        if -acceleration > HARSH_BRAKE {
            once(&mut self.brake_at).then_some(Harsh::Brake)
        } else if acceleration > HARSH_ACCELERATION {
            once(&mut self.accelerate_at).then_some(Harsh::Accelerate)
        } else if lateral > HARSH_CORNER && speed.abs() > 3.0 {
            once(&mut self.corner_at).then_some(Harsh::Corner)
        } else {
            None
        }
    }
}

/// A place on the map (its `places.json`), in the game's world.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub kind: String,
    pub city: String,
    pub city_id: String,
    pub name: Option<String>,
    pub x: f64,
    pub z: f64,
    pub node: Option<String>,
    pub population: u32,
}

/// The places of the map in `folder` (its `map.json` names their file).
pub fn read_places(folder: &Path) -> Vec<Place> {
    let read = || -> Option<Vec<Place>> {
        let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(folder.join("map.json")).ok()?).ok()?;
        let file = folder.join(manifest.get("places")?.as_str()?);
        let places: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(file).ok()?).ok()?;
        let population: std::collections::HashMap<String, u32> = manifest
            .get("labels")
            .and_then(|labels| labels.as_array())
            .map(|labels| {
                labels
                    .iter()
                    .filter_map(|label| {
                        let name = label.get("localized").or_else(|| label.get("name"))?.as_str()?;
                        Some((name.to_owned(), label.get("population")?.as_u64()? as u32))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(places.iter().filter_map(|place| parse_place(place, &population)).collect())
    };
    read().unwrap_or_default()
}

fn parse_place(place: &serde_json::Value, population: &std::collections::HashMap<String, u32>) -> Option<Place> {
    let at = place.get("at")?.as_array()?;
    let (lng, lat) = (at.first()?.as_f64()?, at.get(1)?.as_f64()?);
    let text = |key: &str| place.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    let city = text("city").or_else(|| text("localized")).unwrap_or_default();
    Some(Place {
        kind: text("kind")?,
        population: population.get(&city).copied().unwrap_or(0),
        city,
        city_id: text("cityId").unwrap_or_default(),
        name: text("name"),
        x: lng * westplay_mapdata::area::METRES_PER_DEGREE,
        z: -lat * westplay_mapdata::area::METRES_PER_DEGREE,
        node: text("node"),
    })
}

/// Game km between two points of the world, by road.
pub fn road_km(ax: f64, az: f64, bx: f64, bz: f64) -> f64 {
    (ax - bx).hypot(az - bz) * SCALE / 1000.0 * ROAD_FACTOR
}

/// Game minutes to drive `km`.
pub fn drive_minutes(km: f64) -> f64 {
    km / AVERAGE_KMH * 60.0
}

/// A small, fixed random source (splitmix64): the same trips from the same seed.
#[derive(Clone, Debug)]
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn roll(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// 0 to 1.
    pub fn unit(&mut self) -> f64 {
        (self.roll() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.roll() % n as u64) as usize }
    }
}

/// Where the tours go: a region's cities (map ids), the first the map has is the destination.
const TOURS: &[(&str, &[&str])] = &[
    ("Paris", &["paris"]),
    ("Tirol", &["innsbruck", "salzburg"]),
    ("Italië", &["venezia", "firenze", "roma", "verona", "milano"]),
    ("Praag", &["prague"]),
    ("Scandinavië", &["oslo", "stockholm", "kobenhavn", "goteborg", "bergen"]),
    ("Berlijn", &["berlin"]),
    ("Londen", &["london"]),
    ("Wenen", &["wien"]),
    ("Zwitserland", &["zurich", "bern", "geneve"]),
    ("Côte d'Azur", &["nice", "marseille"]),
];

const SCHOOL: &[&str] = &["berlin", "london", "paris", "brussel", "prague", "koln"];

/// What the catalogue is made from: the map's places, the player's recorded points, the depot, the coach's seats.
pub struct Planning<'a> {
    pub places: &'a [Place],
    pub recorded: &'a [Recorded],
    pub depot: Point,
    pub seats: u32,
}

impl Planning<'_> {
    fn city(&self, id: &str) -> Option<&Place> {
        self.places.iter().find(|place| place.kind == "city" && place.city_id == id)
    }

    fn cities(&self) -> impl Iterator<Item = &Place> {
        self.places.iter().filter(|place| place.kind == "city")
    }

    fn point_of(place: &Place, radius: f64) -> Point {
        let name = match (&place.name, place.kind.as_str()) {
            (Some(name), "parking" | "fuel" | "hotel") if !place.city.is_empty() => format!("{name}, {}", place.city),
            (Some(name), _) => name.clone(),
            (None, _) => place.city.clone(),
        };
        Point { name, x: place.x, z: place.z, radius, node: place.node.clone() }
    }

    /// A place of `kinds` (or a recorded point of `recorded`) nearest `(x, z)`, within `within` metres.
    fn nearest(&self, kinds: &[&str], recorded: StopKind, x: f64, z: f64, within: f64) -> Option<Point> {
        let own = self
            .recorded
            .iter()
            .filter(|r| r.kind == recorded && r.point.distance(x, z) <= within)
            .min_by(|a, b| a.point.distance(x, z).total_cmp(&b.point.distance(x, z)))
            .map(|r| r.point.clone());
        own.or_else(|| {
            self.places
                .iter()
                .filter(|place| kinds.contains(&place.kind.as_str()))
                .map(|place| (place, (place.x - x).hypot(place.z - z)))
                .filter(|(_, d)| *d <= within)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(place, _)| Self::point_of(place, recorded.radius()))
        })
    }

    /// The hotel for a city: one of its own, else the nearest.
    fn hotel(&self, city: &Place) -> Point {
        let own = self.recorded.iter().find(|r| r.kind == StopKind::Hotel && r.point.distance(city.x, city.z) < 4000.0);
        if let Some(own) = own {
            return own.point.clone();
        }
        self.places
            .iter()
            .filter(|place| place.kind == "hotel")
            .min_by(|a, b| {
                let d = |p: &Place| (p.x - city.x).hypot(p.z - city.z) + if p.city_id == city.city_id { 0.0 } else { 1e6 };
                d(a).total_cmp(&d(b))
            })
            .filter(|place| (place.x - city.x).hypot(place.z - city.z) < 6000.0)
            .map(|place| Point {
                name: format!("Hotel {}", place.city),
                ..Self::point_of(place, StopKind::Hotel.radius())
            })
            .unwrap_or_else(|| Point {
                name: format!("Hotel {}", city.city),
                ..Self::point_of(city, StopKind::Hotel.radius())
            })
    }

    /// The pick-up points on the way from the depot towards `(x, z)`: the player's own if they recorded any,
    /// else the depot's town and up to two nearby towns little out of the way.
    fn pickups(&self, x: f64, z: f64) -> Vec<Point> {
        let own: Vec<Point> =
            self.recorded.iter().filter(|r| r.kind == StopKind::Pickup).map(|r| r.point.clone()).collect();
        if !own.is_empty() {
            return own;
        }
        let depot = &self.depot;
        let mut points = vec![Point { name: format!("{} (busstation)", depot.name), ..depot.clone() }];
        let straight = depot.distance(x, z);
        let mut towns: Vec<(&Place, f64)> = self
            .cities()
            .map(|city| (city, depot.distance(city.x, city.z)))
            .filter(|(_, d)| *d > 800.0 && *d < 6000.0)
            .filter(|(city, d)| d + (city.x - x).hypot(city.z - z) - straight < 3000.0)
            .collect();
        towns.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (town, _) in towns.into_iter().take(2) {
            let point = self
                .nearest(&["parking", "fuel"], StopKind::Pickup, town.x, town.z, 1500.0)
                .map(|p| Point { name: format!("{} ({})", town.city, p.name), ..p })
                .unwrap_or_else(|| Self::point_of(town, StopKind::Pickup.radius()));
            points.push(point);
        }
        points
    }

    /// The breaks on the way from `a` to `b` (game minutes of driving between them, from `start`): a coffee stop
    /// at most every [`COFFEE_EVERY`], one near noon a meal; each at a parking or fuel station near the line.
    fn breaks(&self, a: (f64, f64), b: (f64, f64), minutes: f64, start: f64) -> Vec<(Point, StopKind, f64)> {
        let count = ((minutes / COFFEE_EVERY).ceil() as usize).saturating_sub(1);
        let mut out: Vec<(Point, StopKind, f64)> = Vec::new();
        let mut dwelt = 0.0;
        let mut meal_done = false;
        for i in 1..=count {
            let share = i as f64 / (count + 1) as f64;
            let (x, z) = (a.0 + (b.0 - a.0) * share, a.1 + (b.1 - a.1) * share);
            let when = start + minutes * share + dwelt;
            let hour = (when % MINUTES_PER_DAY) / 60.0;
            let kind = if !meal_done && (11.5..=14.5).contains(&hour) {
                meal_done = true;
                StopKind::Meal
            } else {
                StopKind::Coffee
            };
            let point = self
                .nearest(&["parking", "fuel"], StopKind::Coffee, x, z, 3000.0)
                .or_else(|| self.nearest(&["parking", "fuel", "service"], StopKind::Coffee, x, z, 12000.0));
            if let Some(point) = point {
                dwelt += kind.dwell();
                out.push((point, kind, when));
            }
        }
        out
    }

    /// A leg's driving from `from` to `to` (planned from `clock`, on `day`): its breaks, and an overnight at a
    /// hotel on the way when it's more than a day's driving. Returns the game km.
    fn drive(
        &self,
        stops: &mut Vec<Stop>,
        from: (f64, f64),
        to: (f64, f64),
        leg: Leg,
        clock: &mut f64,
        day: &mut u32,
    ) -> f64 {
        let km = road_km(from.0, from.1, to.0, to.1);
        let minutes = drive_minutes(km);
        let parts = (minutes / DAY_DRIVING).ceil().max(1.0) as usize;
        let mut start = from;
        for part in 0..parts {
            let share = (part + 1) as f64 / parts as f64;
            let end = (from.0 + (to.0 - from.0) * share, from.1 + (to.1 - from.1) * share);
            let piece = minutes / parts as f64;
            let breaks = self.breaks(start, end, piece, *clock);
            let dwell: f64 = breaks.iter().map(|(_, kind, _)| kind.dwell()).sum();
            for (point, kind, at) in breaks {
                stops.push(Stop::new(kind, leg, point, at, *day));
            }
            *clock += piece + dwell;
            if part + 1 < parts {
                // An overnight on the way: the town with a hotel nearest where the day ends.
                let town = self
                    .cities()
                    .filter(|c| self.places.iter().any(|p| p.kind == "hotel" && p.city_id == c.city_id))
                    .min_by(|a, b| ((a.x - end.0).hypot(a.z - end.1)).total_cmp(&(b.x - end.0).hypot(b.z - end.1)));
                if let Some(town) = town {
                    stops.push(Stop::new(StopKind::Hotel, leg, self.hotel(town), *clock, *day));
                }
                *day += 1;
                *clock = next_morning(*clock, 8.5);
            }
            start = end;
        }
        km
    }

    /// One trip of `kind` to `destination` (a city; `title` what the catalogue calls it), `days` long, leaving at
    /// `departure`, its stops planned.
    #[allow(clippy::too_many_arguments)]
    pub fn trip(
        &self,
        id: u64,
        kind: TripKind,
        title: &str,
        destination: &Place,
        days: u32,
        departure: f64,
        listed: f64,
        dice: &mut Dice,
    ) -> Trip {
        let mut stops: Vec<Stop> = Vec::new();
        let mut clock = departure;
        let mut day = 1u32;
        let mut distance = 0.0;
        let pickups = self.pickups(destination.x, destination.z);
        let seats = self.seats.max(8);
        // Each pick-up point's share of the passengers (thousandths): the home town's the most.
        let share = |i: usize| {
            let n = pickups.len() as f64;
            ((if i == 0 { 1.0 / n + 0.15 } else { 0.85 / n }) * 1000.0) as u32
        };
        let mut at = (pickups[0].x, pickups[0].z);
        for (i, point) in pickups.iter().enumerate() {
            let km = road_km(at.0, at.1, point.x, point.z);
            distance += km;
            clock += drive_minutes(km);
            let mut stop = Stop::new(StopKind::Pickup, Leg::Boarding, point.clone(), clock, day);
            stop.passengers = share(i);
            stops.push(stop);
            clock += StopKind::Pickup.dwell();
            at = (point.x, point.z);
        }
        let there = (destination.x, destination.z);
        distance += self.drive(&mut stops, at, there, Leg::Outbound, &mut clock, &mut day);
        let transit = day - 1;
        let dest_point =
            Point { name: destination.city.clone(), ..Self::point_of(destination, StopKind::Destination.radius()) };
        let days = if days <= 1 && transit == 0 { 1 } else { days.max(2 * transit + 2) };
        if days == 1 {
            // A day out: the destination, a few hours there.
            stops.push(Stop::new(StopKind::Destination, Leg::Stay, dest_point, clock, day));
            clock += 4.0 * 60.0;
        } else {
            let hotel = self.hotel(destination);
            stops.push(Stop::new(StopKind::Hotel, Leg::Stay, hotel.clone(), clock, day));
            // The days there: an excursion each (a town near, or the city itself), back at the hotel by evening.
            let mut near: Vec<&Place> = self
                .cities()
                .filter(|c| c.city_id != destination.city_id)
                .filter(|c| (c.x - there.0).hypot(c.z - there.1) < 9000.0)
                .collect();
            near.sort_by(|a, b| ((a.x - there.0).hypot(a.z - there.1)).total_cmp(&(b.x - there.0).hypot(b.z - there.1)));
            let leaving = days - transit;
            for (excursion, d) in (day + 1..leaving).enumerate() {
                day = d;
                clock = next_morning(clock, 9.5);
                let place = near.get(excursion).copied().unwrap_or(destination);
                let point = if place.city_id == destination.city_id {
                    Point { name: format!("{} centrum", destination.city), ..dest_point.clone() }
                } else {
                    Self::point_of(place, StopKind::Excursion.radius())
                };
                let km = road_km(there.0, there.1, point.x, point.z);
                distance += 2.0 * km;
                stops.push(Stop::new(StopKind::Excursion, Leg::Stay, point, clock + drive_minutes(km), day));
                clock += 2.0 * drive_minutes(km) + 5.0 * 60.0;
                stops.push(Stop::new(StopKind::Hotel, Leg::Stay, hotel.clone(), clock, day));
            }
            day = leaving;
            clock = next_morning(clock, 9.0);
        }
        // Home: the pick-up points the other way round (the last nearest), then the depot.
        let last = pickups.last().map_or((self.depot.x, self.depot.z), |p| (p.x, p.z));
        distance += self.drive(&mut stops, there, last, Leg::Return, &mut clock, &mut day);
        let mut from = last;
        for (i, point) in pickups.iter().enumerate().rev() {
            let km = road_km(from.0, from.1, point.x, point.z);
            distance += km;
            clock += drive_minutes(km);
            let mut stop = Stop::new(StopKind::Dropoff, Leg::Unloading, point.clone(), clock, day);
            stop.point.radius = point.radius.max(StopKind::Dropoff.radius());
            stop.passengers = share(i);
            stops.push(stop);
            clock += StopKind::Dropoff.dwell();
            from = (point.x, point.z);
        }
        let km = road_km(from.0, from.1, self.depot.x, self.depot.z);
        distance += km;
        stops.push(Stop::new(StopKind::Depot, Leg::ToDepot, self.depot.clone(), clock + drive_minutes(km), day));

        Trip {
            id,
            kind,
            title: title.to_owned(),
            destination: destination.city.clone(),
            days,
            departure,
            listed,
            price: price(kind, days, distance, seats, dice),
            seats,
            booked: 0,
            distance,
            stops,
        }
    }

    /// A catalogue of `count` trips leaving one to five days after `now`.
    pub fn catalogue(&self, now: f64, first_id: u64, count: usize, dice: &mut Dice) -> Vec<Trip> {
        let mut trips = Vec::new();
        let depot = &self.depot;
        let km_to = |c: &Place| road_km(depot.x, depot.z, c.x, c.z);
        let mut days_out: Vec<&Place> = self.cities().filter(|c| (60.0..=320.0).contains(&km_to(c))).collect();
        days_out.sort_by_key(|c| std::cmp::Reverse(c.population));
        if days_out.is_empty() {
            // No town in a day's reach: the nearest further ones.
            days_out = self.cities().filter(|c| km_to(c) > 40.0).collect();
            days_out.sort_by(|a, b| km_to(a).total_cmp(&km_to(b)));
            days_out.truncate(12);
        }
        let tours: Vec<(&str, &Place)> = TOURS
            .iter()
            .filter_map(|(region, ids)| {
                let place = ids.iter().find_map(|id| self.city(id))?;
                (km_to(place) > 250.0).then_some((*region, place))
            })
            .collect();
        let schools: Vec<&Place> = SCHOOL.iter().filter_map(|id| self.city(id)).filter(|c| km_to(c) > 150.0).collect();
        let pick = |pool: &[&Place], dice: &mut Dice, top: usize| -> usize { dice.below(pool.len().min(top)) };
        let mut id = first_id;
        for slot in 0..count {
            let departure =
                next_morning(now, 7.0 + 0.5 * dice.below(5) as f64) + MINUTES_PER_DAY * dice.below(5) as f64;
            let made = match slot % 8 {
                1 | 4 if !tours.is_empty() => {
                    let (region, place) = tours[dice.below(tours.len())];
                    Some((TripKind::Tour, region.to_owned(), place, 4 + dice.below(5) as u32))
                }
                2 if !schools.is_empty() => {
                    let place = schools[pick(&schools, dice, 6)];
                    Some((TripKind::School, place.city.clone(), place, 3))
                }
                7 if !days_out.is_empty() => {
                    let place = days_out[pick(&days_out, dice, 30)];
                    Some((TripKind::Charter, place.city.clone(), place, 1))
                }
                _ if !days_out.is_empty() => {
                    let place = days_out[pick(&days_out, dice, 12)];
                    Some((TripKind::DayTrip, place.city.clone(), place, 1))
                }
                _ => None,
            };
            if let Some((kind, title, place, days)) = made {
                trips.push(self.trip(id, kind, &title, place, days, departure, now, dice));
                id += 1;
            }
        }
        trips.sort_by(|a, b| a.departure.total_cmp(&b.departure));
        trips
    }
}

/// `hour` o'clock the day after the one `clock` is in (game minutes).
fn next_morning(clock: f64, hour: f64) -> f64 {
    ((clock / MINUTES_PER_DAY).floor() + 1.0) * MINUTES_PER_DAY + hour * 60.0
}

/// A seat's price (a charter's and a school's: the contract), by the kind, days and distance, rounded to a
/// catalogue's price.
fn price(kind: TripKind, days: u32, km: f64, seats: u32, dice: &mut Dice) -> f64 {
    let noise = 0.92 + 0.16 * dice.unit();
    let raw = match kind {
        TripKind::DayTrip => (29.0 + km * 0.07) * noise,
        TripKind::Tour => (95.0 * f64::from(days) + km * 0.06) * noise,
        TripKind::School => (f64::from(seats) * (45.0 * f64::from(days) + km * 0.03) * 0.85) * noise,
        TripKind::Charter => (650.0 + km * 2.4) * noise,
    };
    match kind {
        TripKind::School | TripKind::Charter => (raw / 50.0).round() * 50.0,
        _ => ((raw / 5.0).round() * 5.0 - 1.0).max(4.0),
    }
}

/// A seat's reference price: what passengers think fair.
fn fair_price(trip: &Trip) -> f64 {
    match trip.kind {
        TripKind::DayTrip => 29.0 + trip.distance * 0.07,
        TripKind::Tour => 95.0 * f64::from(trip.days) + trip.distance * 0.06,
        _ => trip.price,
    }
}

/// The season, by the month (1 to 12): summer and the holidays book best.
pub fn season(month: u32) -> f64 {
    match month {
        6..=8 => 1.25,
        4 | 5 | 9 => 1.1,
        12 => 1.05,
        10 => 1.0,
        _ => 0.85,
    }
}

/// The seats booked at `now`: the demand (reputation, price against fair, season, a trip's own luck) times how
/// far the booking period has come (from listing to departure). A charter or school trip is booked whole once
/// the reputation will do.
pub fn bookings(trip: &Trip, reputation: f64, now: f64, month: u32) -> u32 {
    let seats = f64::from(trip.seats);
    if matches!(trip.kind, TripKind::Charter | TripKind::School) {
        let wanted = if trip.kind == TripKind::School { 25.0 } else { 0.0 };
        return if reputation >= wanted { trip.seats } else { 0 };
    }
    let mut dice = Dice::new(trip.id.wrapping_mul(0x2545_f491_4f6c_dd1d));
    let luck = 0.85 + 0.3 * dice.unit();
    let base = 0.3 + 0.55 * (reputation / 100.0).clamp(0.0, 1.0);
    let price = (fair_price(trip) / trip.price.max(1.0)).powf(1.5).clamp(0.3, 1.6);
    let demand = (base * price * season(month) * luck).clamp(0.0, 1.0);
    let period = (trip.departure - trip.listed).max(60.0);
    let progress = ((now - trip.listed) / period).clamp(0.0, 1.0);
    // Most book early, the rest in the last days.
    let progress = 1.0 - (1.0 - progress).powf(1.6);
    (seats * demand * (0.25 + 0.75 * progress)).floor() as u32
}

/// The month now on the PC's clock (1 to 12).
pub fn month_now() -> u32 {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0) as i64;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    (if mp < 10 { mp + 3 } else { mp - 9 }) as u32
}

/// Fills in a trip's passengers per pick-up point from its bookings (the shares planned in thousandths).
fn seat_passengers(trip: &mut Trip) {
    let booked = trip.booked;
    let pickups: Vec<usize> =
        trip.stops.iter().enumerate().filter(|(_, s)| s.kind == StopKind::Pickup).map(|(i, _)| i).collect();
    let shares: Vec<u32> = pickups.iter().map(|&i| trip.stops[i].passengers).collect();
    let total: u32 = shares.iter().sum::<u32>().max(1);
    let mut given = 0;
    let mut counts = Vec::new();
    for (n, share) in shares.iter().enumerate() {
        let count = if n + 1 == shares.len() { booked - given } else { (booked * share / total).min(booked - given) };
        given += count;
        counts.push(count);
    }
    for (&index, &count) in pickups.iter().zip(&counts) {
        trip.stops[index].passengers = count;
    }
    // Drop-offs: the same people, the same places.
    let names: Vec<(String, u32)> =
        pickups.iter().zip(&counts).map(|(&i, &c)| (trip.stops[i].point.name.clone(), c)).collect();
    for stop in trip.stops.iter_mut().filter(|s| s.kind == StopKind::Dropoff) {
        stop.passengers = names.iter().find(|(name, _)| *name == stop.point.name).map_or(0, |(_, c)| *c);
    }
}

/// The company's book: its balance and reputation, the catalogue, the trip driven, the history, the player's
/// points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Book {
    pub balance: f64,
    pub reputation: f64,
    pub catalogue: Vec<Trip>,
    pub active: Option<Active>,
    pub history: Vec<Finished>,
    pub recorded: Vec<Recorded>,
    pub next_id: u64,
    /// The seed of the next catalogue.
    pub seed: u64,
}

impl Default for Book {
    fn default() -> Self {
        Self {
            balance: 25_000.0,
            reputation: 50.0,
            catalogue: Vec::new(),
            active: None,
            history: Vec::new(),
            recorded: Vec::new(),
            next_id: 1,
            seed: 0x5eed,
        }
    }
}

impl Book {
    pub fn path(dir: &Path) -> PathBuf {
        dir.join("touring").join("book.json")
    }

    pub fn load(dir: &Path) -> Self {
        let path = Self::path(dir);
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                let _ = std::fs::rename(&path, path.with_extension("json.bad"));
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        let path = Self::path(dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let aside = path.with_extension("json.new");
        std::fs::write(&aside, serde_json::to_vec(self).map_err(std::io::Error::other)?)?;
        std::fs::rename(aside, path)
    }

    /// The trip ended (at the depot, or called off at `now`): paid, its mood counted into the reputation,
    /// kept in the history.
    pub fn settle(&mut self, now: f64, cancelled: bool) -> Option<Finished> {
        let active = self.active.take()?;
        let trip = &active.trip;
        let income = if cancelled { 0.0 } else { trip.income() };
        let mut costs = costs(trip, &active.meter);
        if cancelled {
            // Only the days driven, no hotels booked through.
            let driven = ((now - trip.departure) / MINUTES_PER_DAY).ceil().clamp(0.0, f64::from(trip.days));
            costs.wage = driven * WAGE_PER_DAY;
            costs.hotel = 0.0;
        }
        let mood = if cancelled { 0.0 } else { mood(&active.score).total };
        let step = if cancelled { -5.0 } else { reputation_step(mood) };
        self.reputation = (self.reputation + step).clamp(0.0, 100.0);
        let profit = income - costs.total();
        self.balance += profit;
        let finished = Finished {
            id: trip.id,
            kind: trip.kind,
            destination: trip.destination.clone(),
            days: trip.days,
            departure: trip.departure,
            ended: now,
            passengers: trip.booked,
            income,
            costs,
            profit,
            mood,
            reputation: self.reputation,
            cancelled,
        };
        self.history.insert(0, finished.clone());
        self.history.truncate(KEEP_HISTORY);
        Some(finished)
    }

    /// The catalogue at `now`: bookings brought up to date, trips missed taken out (reputation suffers when they
    /// had bookings), new ones added up to [`CATALOGUE`].
    pub fn refresh(&mut self, planning: Option<&Planning>, now: f64, month: u32) {
        let reputation = self.reputation;
        for trip in &mut self.catalogue {
            let at = now.min(trip.departure);
            trip.booked = bookings(trip, reputation, at, month).max(trip.booked).min(trip.seats);
        }
        let mut missed = 0;
        self.catalogue.retain(|trip| {
            let gone = now > trip.departure + 120.0;
            if gone && trip.booked > 0 {
                missed += 1;
            }
            !gone
        });
        self.reputation = (self.reputation - 2.0 * f64::from(missed)).max(0.0);
        if let Some(planning) = planning
            && self.catalogue.len() < CATALOGUE
        {
            let mut dice = Dice::new(self.seed);
            self.seed = dice.roll();
            let added = planning.catalogue(now, self.next_id, CATALOGUE - self.catalogue.len(), &mut dice);
            self.next_id += added.len() as u64 + 1;
            self.catalogue.extend(added);
            self.catalogue.sort_by(|a, b| a.departure.total_cmp(&b.departure));
        }
    }

    /// A trip from the catalogue taken: the active trip, its passengers seated.
    pub fn start(&mut self, id: u64) -> bool {
        if self.active.is_some() {
            return false;
        }
        let Some(index) = self.catalogue.iter().position(|trip| trip.id == id) else { return false };
        let mut trip = self.catalogue.remove(index);
        seat_passengers(&mut trip);
        self.active = Some(Active::new(trip));
        true
    }
}

/// An announcement Dashly asks the driver for: what (`welcome`, `coffee`, `meal`, `nextPickup`, `arrival`,
/// `hotel`, `return`, `goodbye`), where, and when (game minutes, for "expected at").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub kind: String,
    pub place: String,
    pub at: Option<f64>,
    /// When asked (game minutes): given up on after a while.
    pub asked: f64,
}

/// How long an announcement is waited for (game minutes).
const PROMPT_FOR: f64 = 30.0;

/// The announcement due at this point of the trip, if one is (and wasn't asked before): `eta` the game minutes
/// to the next stop.
pub fn prompt_due(active: &Active, eta: Option<f64>, just_reached: Option<&Stop>, now: f64) -> Option<Prompt> {
    let key = |kind: &str| format!("{kind}:{}", active.next);
    let fresh = |kind: &str| !active.prompted.contains(&key(kind));
    let ask = |kind: &str, place: &str, at: Option<f64>| {
        fresh(kind).then(|| Prompt { kind: kind.to_owned(), place: place.to_owned(), at, asked: now })
    };
    let next = active.next_stop();
    if let Some(reached) = just_reached {
        let first_pickup = reached.kind == StopKind::Pickup && active.next == 1;
        return match (reached.kind, next) {
            (StopKind::Pickup, _) if first_pickup => ask("welcome", &active.trip.destination, None),
            (StopKind::Pickup, Some(n)) if n.kind == StopKind::Pickup => ask("nextPickup", &n.point.name, Some(n.at)),
            (StopKind::Pickup, _) => ask("departure", &active.trip.destination, None),
            (StopKind::Destination | StopKind::Excursion, _) => ask("arrival", &reached.point.name, None),
            (StopKind::Hotel, _) => ask("hotel", &reached.point.name, None),
            (StopKind::Dropoff, Some(n)) if n.kind == StopKind::Depot => ask("goodbye", &reached.point.name, None),
            _ => None,
        };
    }
    let next = next?;
    match (next.kind, eta) {
        (StopKind::Coffee, Some(eta)) if eta <= 10.0 => ask("coffee", &next.point.name, None),
        (StopKind::Meal, Some(eta)) if eta <= 10.0 => ask("meal", &next.point.name, None),
        (StopKind::Dropoff, Some(eta)) if eta <= 10.0 && next.leg == Leg::Unloading => ask("return", &next.point.name, None),
        _ => None,
    }
}

/// Where the game's GPS is to take the coach next: a point, and its map node when known.
#[derive(Clone, Debug, PartialEq)]
pub struct Navigate {
    pub x: f64,
    pub z: f64,
    pub node: Option<u64>,
}

/// The company as the companion sees it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TouringStatus {
    pub company: String,
    pub depot: Option<String>,
    pub balance: f64,
    pub reputation: f64,
    pub catalogue: Vec<Trip>,
    pub active: Option<ActiveStatus>,
    pub history: Vec<Finished>,
    pub recorded: Vec<Recorded>,
    /// The game's clock now (game minutes; `None`: no game).
    pub now: Option<f64>,
    /// Whether the map's places are known (the catalogue can be made).
    pub map_ready: bool,
}

/// The trip driven, as the companion and the cab see it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveStatus {
    pub trip: Trip,
    pub next: usize,
    pub phase: Phase,
    pub on_board: u32,
    pub to_collect: u32,
    pub mood: Mood,
    pub score: Score,
    pub meter: Meter,
    /// To the next stop: game km (straight line by road), game minutes, and how far ahead (negative) or behind
    /// the plan the coach will be there.
    pub distance: Option<f64>,
    pub eta: Option<f64>,
    pub late: Option<f64>,
    pub prompt: Option<Prompt>,
}

/// The company as the host runs it: its book, the map's places, the detectors, what it tells the companion and
/// the cab, the next stop for the GPS.
pub struct Touring {
    book: Book,
    places: Vec<Place>,
    places_of: Option<PathBuf>,
    arrival: Arrival,
    comfort: Comfort,
    last_tick: Instant,
    saved: Instant,
    dirty: bool,
    /// The last position and game time seen (a ferry or train: the world jumps).
    last_seen: Option<(f64, f64, u32)>,
    last_fuel: Option<f32>,
    /// The tachograph's driving since the last break, as last read.
    last_driving: Option<f64>,
    overrun_counted: bool,
    /// The PA heard since the prompt was asked (real seconds).
    pa_for: f64,
    navigate_to: Option<usize>,
    navigation: Option<Navigate>,
    sent: Option<(String, Instant)>,
}

impl Touring {
    pub fn start(dir: &Path) -> Self {
        let now = Instant::now();
        Self {
            book: Book::load(dir),
            places: Vec::new(),
            places_of: None,
            arrival: Arrival::default(),
            comfort: Comfort::default(),
            last_tick: now - Duration::from_secs(5),
            saved: now,
            dirty: false,
            last_seen: None,
            last_fuel: None,
            last_driving: None,
            overrun_counted: false,
            pa_for: 0.0,
            navigate_to: None,
            navigation: None,
            sent: None,
        }
    }

    pub fn due(&self) -> bool {
        self.last_tick.elapsed() >= Duration::from_secs(1)
    }

    /// The map shown, in `folder`: its places read (once each map).
    pub fn map(&mut self, folder: Option<PathBuf>) {
        if folder != self.places_of {
            self.places = folder.as_deref().map(read_places).unwrap_or_default();
            crate::log!("[touring] {} places on the map", self.places.len());
            self.places_of = folder;
        }
    }

    pub fn save(&mut self, dir: &Path) {
        self.saved = Instant::now();
        self.dirty = false;
        if let Err(e) = self.book.save(dir) {
            crate::log!("[touring] couldn't keep the book: {e}");
        }
    }

    /// Where the GPS is to go, once each new stop (taken).
    pub fn take_navigation(&mut self) -> Option<Navigate> {
        self.navigation.take()
    }

    /// The telemetry as it comes (every 50 ms): comfort, fuel, ferries.
    pub fn observe(&mut self, t: &westplay_shared::Telemetry, now: Instant) {
        let Some(active) = self.book.active.as_mut() else {
            self.last_seen = None;
            self.last_fuel = None;
            return;
        };
        if t.paused {
            return;
        }
        if active.on_board > 0 {
            match self.comfort.update(t.speed, t.heading, now) {
                Some(Harsh::Brake) => active.score.harsh_brakes += 1,
                Some(Harsh::Accelerate) => active.score.harsh_accelerations += 1,
                Some(Harsh::Corner) => active.score.harsh_corners += 1,
                None => {}
            }
        }
        if let Some(before) = self.last_fuel.replace(t.fuel)
            && t.fuel < before
            && before - t.fuel < 5.0
        {
            active.meter.litres += f64::from(before - t.fuel);
        }
        if let Some((x, z, time)) = self.last_seen.replace((t.x, t.z, t.game_time)) {
            let moved = (t.x - x).hypot(t.z - z);
            let scale = if t.local_scale > 0.0 { f64::from(t.local_scale) } else { SCALE };
            if moved > 1500.0 && t.game_time > time + 30 {
                // A ferry or a train: the world jumped, hours on.
                active.meter.ferries += 1;
                self.dirty = true;
            } else if moved < 200.0 {
                active.meter.km += moved * scale / 1000.0;
            }
        }
    }

    /// A tick (about once a second): the actions asked for, the catalogue's bookings, the active trip's stops.
    /// `driving_since_break`: the tachograph's (minutes; `None`: off). `pa`: the coach's PA heard now. Returns the
    /// companion's status and a message for the cab when there's news.
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        dir: &Path,
        settings: &crate::settings::TouringSettings,
        telemetry: Option<&westplay_shared::Telemetry>,
        driving_since_break: Option<f64>,
        pa: bool,
    ) -> (Option<TouringStatus>, Option<String>) {
        let real_now = Instant::now();
        let elapsed = real_now.duration_since(self.last_tick).as_secs_f64().min(5.0);
        self.last_tick = real_now;
        if !settings.enabled {
            return (None, None);
        }
        let playing = telemetry.filter(|t| !t.paused);
        let game_now = telemetry.map(|t| f64::from(t.game_time));
        let actions: Vec<Action> = ACTIONS.lock().map(|mut a| std::mem::take(&mut *a)).unwrap_or_default();
        let depot = self.depot(settings);
        for action in actions {
            self.dirty = true;
            match action {
                Action::Start { id } => {
                    if self.book.start(id) {
                        crate::log!("[touring] trip {id} started");
                        self.arrival.reset();
                        self.navigate_to = None;
                    }
                }
                Action::Cancel => {
                    if let Some(f) = self.book.settle(game_now.unwrap_or(0.0), true) {
                        crate::log!("[touring] trip {} called off", f.id);
                    }
                }
                Action::Refresh => {
                    self.book.catalogue.retain(|trip| trip.booked > 0);
                }
                Action::Record { kind, name } => {
                    if let Some(t) = telemetry {
                        let name = if name.trim().is_empty() { format!("{kind:?}") } else { name.trim().to_owned() };
                        let radius = if kind == StopKind::Hotel { 200.0 } else { 100.0 };
                        let point = Point { name, x: t.x, z: t.z, radius, node: None };
                        self.book.recorded.push(Recorded { kind, point });
                    }
                }
                Action::Forget { index } => {
                    if index < self.book.recorded.len() {
                        self.book.recorded.remove(index);
                    }
                }
                Action::Skip => {
                    if let Some(active) = &mut self.book.active {
                        active.skip();
                        self.arrival.reset();
                    }
                }
                Action::Reset => {
                    let recorded = std::mem::take(&mut self.book.recorded);
                    self.book = Book { recorded, ..Book::default() };
                }
            }
        }

        if let Some(now) = game_now {
            let recorded = self.book.recorded.clone();
            let planning = depot.clone().filter(|_| !self.places.is_empty()).map(|depot| Planning {
                places: &self.places,
                recorded: &recorded,
                depot,
                seats: settings.seats,
            });
            let before = self.book.catalogue.len();
            self.book.refresh(planning.as_ref(), now, month_now());
            if self.book.catalogue.len() != before {
                self.dirty = true;
            }
        }

        let mut just_reached: Option<Stop> = None;
        let mut finished = false;
        if let (Some(active), Some(t)) = (self.book.active.as_mut(), playing) {
            let now = f64::from(t.game_time);
            // Leaving a coffee stop: its length.
            if let Some(index) = active.resting_at {
                let point = &active.trip.stops[index].point;
                if point.distance(t.x, t.z) > point.radius {
                    active.leave_rest(now);
                    self.dirty = true;
                }
            }
            if let Some(stop) = active.next_stop()
                && self.arrival.update(&stop.point, t.x, t.z, t.speed, real_now)
            {
                just_reached = active.reach(now);
                self.arrival.reset();
                self.dirty = true;
                if let Some(stop) = &just_reached {
                    crate::log!("[touring] reached {:?} {}", stop.kind, stop.point.name);
                }
                finished = active.next_stop().is_none();
            }
            // The tachograph: breaks and the 4.5 hours, with passengers on board.
            if let Some(driving) = driving_since_break {
                if active.on_board > 0 {
                    if driving > crate::tacho::CONTINUOUS_DRIVING && !self.overrun_counted {
                        active.score.overruns += 1;
                        self.overrun_counted = true;
                    }
                    if self.last_driving.is_some_and(|before| before >= 30.0) && driving < 1.0 {
                        active.score.breaks += 1;
                    }
                }
                if driving < 1.0 {
                    self.overrun_counted = false;
                }
                self.last_driving = Some(driving);
            }
            // Announcements: the PA heard for a few seconds while one is asked.
            if let Some(prompt) = &active.prompt {
                if pa {
                    self.pa_for += elapsed;
                }
                if self.pa_for >= 3.0 {
                    active.score.announcements += 1;
                    active.prompt = None;
                    self.dirty = true;
                } else if now - prompt.asked > PROMPT_FOR {
                    active.prompt = None;
                }
            }
            if !finished {
                let eta = active.next_stop().map(|s| drive_minutes(road_km(t.x, t.z, s.point.x, s.point.z)));
                if (active.prompt.is_none() || just_reached.is_some())
                    && settings.announcements
                    && let Some(prompt) = prompt_due(active, eta, just_reached.as_ref(), now)
                {
                    active.prompted.push(format!("{}:{}", prompt.kind, active.next));
                    active.prompt = Some(prompt);
                    self.pa_for = 0.0;
                }
            }
            if self.navigate_to != Some(active.next) {
                self.navigate_to = Some(active.next);
                self.navigation = active.next_stop().map(|stop| Navigate {
                    x: stop.point.x,
                    z: stop.point.z,
                    node: stop.point.node.as_deref().and_then(|n| u64::from_str_radix(n, 16).ok()),
                });
            }
        }
        if finished && let Some(f) = self.book.settle(game_now.unwrap_or(0.0), false) {
            crate::log!("[touring] trip {} done: mood {:.0}, profit {:.0}", f.id, f.mood, f.profit);
            self.navigate_to = None;
            self.dirty = true;
        }
        if (self.dirty && self.saved.elapsed() >= Duration::from_secs(2)) || self.saved.elapsed() >= Duration::from_secs(60) {
            self.save(dir);
        }

        let active = self.book.active.as_ref().map(|active| {
            let next = active.next_stop();
            let (distance, eta) = match (next, playing) {
                (Some(stop), Some(t)) => {
                    let km = road_km(t.x, t.z, stop.point.x, stop.point.z);
                    (Some(km), Some(drive_minutes(km)))
                }
                _ => (None, None),
            };
            let late = match (next, eta, game_now) {
                (Some(stop), Some(eta), Some(now)) => Some(now + eta - stop.at),
                _ => None,
            };
            ActiveStatus {
                trip: active.trip.clone(),
                next: active.next,
                phase: active.phase(),
                on_board: active.on_board,
                to_collect: active.to_collect(),
                mood: mood(&active.score),
                score: active.score.clone(),
                meter: active.meter.clone(),
                distance,
                eta,
                late,
                prompt: active.prompt.clone(),
            }
        });
        let message = self.message(settings, active.as_ref(), game_now);
        let status = TouringStatus {
            company: settings.company.clone(),
            depot: depot.map(|d| d.name),
            balance: self.book.balance,
            reputation: self.book.reputation,
            catalogue: self.book.catalogue.clone(),
            active,
            history: self.book.history.clone(),
            recorded: self.book.recorded.clone(),
            now: game_now,
            map_ready: !self.places.is_empty(),
        };
        (Some(status), message)
    }

    /// The depot: the player's own (recorded), the home town chosen, or the first Dutch town the map has.
    fn depot(&self, settings: &crate::settings::TouringSettings) -> Option<Point> {
        if let Some(own) = self.book.recorded.iter().rev().find(|r| r.kind == StopKind::Depot) {
            return Some(own.point.clone());
        }
        let ids: Vec<&str> = if settings.home.is_empty() {
            vec!["amsterdam", "rotterdam", "groningen", "eindhoven"]
        } else {
            vec![settings.home.as_str()]
        };
        let city = ids.iter().find_map(|id| {
            self.places.iter().find(|p| p.kind == "city" && (p.city_id == *id || p.city.eq_ignore_ascii_case(id)))
        })?;
        let planning = Planning { places: &self.places, recorded: &[], depot: Planning::point_of(city, 250.0), seats: 0 };
        Some(
            planning
                .nearest(&["garage", "parking"], StopKind::Depot, city.x, city.z, 4000.0)
                .map(|p| Point { name: city.city.clone(), radius: StopKind::Depot.radius(), ..p })
                .unwrap_or_else(|| Planning::point_of(city, StopKind::Depot.radius())),
        )
    }

    /// The cab's driver's terminal: the trip, the next stop, passengers, the mood, the announcement asked for;
    /// sent when it changes, and every few seconds.
    fn message(
        &mut self,
        settings: &crate::settings::TouringSettings,
        active: Option<&ActiveStatus>,
        now: Option<f64>,
    ) -> Option<String> {
        let trip = active.map(|a| {
            let next = a.trip.stops.get(a.next);
            let stop_left = a.trip.stops.len().saturating_sub(a.next);
            serde_json::json!({
                "kind": a.trip.kind,
                "destination": a.trip.destination,
                "days": a.trip.days,
                "day": next.map_or(a.trip.days, |s| s.day),
                "phase": a.phase,
                "next": next.map(|s| serde_json::json!({
                    "kind": s.kind, "name": s.point.name, "at": s.at,
                    "distance": a.distance.map(|d| (d * 10.0).round() / 10.0),
                    "eta": a.eta.map(f64::round),
                    "late": a.late.map(f64::round),
                })),
                "stopsLeft": stop_left,
                "onBoard": a.on_board,
                "toCollect": a.to_collect,
                "seats": a.trip.seats,
                "mood": a.mood.total.round(),
                "prompt": a.prompt,
            })
        });
        let message = serde_json::json!({
            "type": "touring",
            "company": settings.company,
            "now": now,
            "trip": trip,
        })
        .to_string();
        let resend = self.sent.as_ref().is_none_or(|(sent, at)| *sent != message || at.elapsed().as_secs() >= 5);
        resend.then(|| {
            self.sent = Some((message.clone(), Instant::now()));
            message
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(kind: &str, id: &str, x: f64, z: f64) -> Place {
        Place {
            kind: kind.into(),
            city: id.to_uppercase(),
            city_id: id.into(),
            name: None,
            x,
            z,
            node: Some("1a".into()),
            population: 100_000,
        }
    }

    /// A small map: home at the origin, two towns near, Paris far south, a hotel and parkings.
    fn places() -> Vec<Place> {
        vec![
            place("city", "amsterdam", 0.0, 0.0),
            place("city", "rotterdam", 0.0, 1800.0),
            place("city", "utrecht", 1500.0, 1000.0),
            place("city", "paris", 0.0, 21_000.0),
            place("city", "lille", 0.0, 14_000.0),
            place("city", "reims", 4000.0, 18_000.0),
            place("hotel", "paris", 300.0, 21_200.0),
            place("parking", "", 100.0, 7000.0),
            place("parking", "", -200.0, 12_500.0),
            place("fuel", "", 100.0, 3600.0),
            place("parking", "", 0.0, 17_500.0),
            place("garage", "amsterdam", 200.0, 100.0),
        ]
    }

    fn planning(places: &[Place]) -> Planning<'_> {
        Planning {
            places,
            recorded: &[],
            depot: Point { name: "Amsterdam".into(), x: 0.0, z: 0.0, radius: 250.0, node: None },
            seats: 50,
        }
    }

    fn paris_tour(places: &[Place]) -> Trip {
        let planning = planning(places);
        let paris = places.iter().find(|p| p.city_id == "paris").unwrap();
        planning.trip(7, TripKind::Tour, "Paris", paris, 5, 8.0 * 60.0, 0.0, &mut Dice::new(1))
    }

    #[test]
    fn a_stop_counts_once_still_inside_its_radius_for_a_few_seconds() {
        let point = Point { name: "Opstap".into(), x: 100.0, z: 100.0, radius: 50.0, node: None };
        let start = Instant::now();
        let mut arrival = Arrival::default();
        // Driving through: never.
        assert!(!arrival.update(&point, 100.0, 100.0, 8.0, start));
        assert!(!arrival.update(&point, 100.0, 100.0, 8.0, start + Duration::from_secs(10)));
        // Standing outside the radius: never.
        assert!(!arrival.update(&point, 200.0, 100.0, 0.0, start));
        assert!(!arrival.update(&point, 200.0, 100.0, 0.0, start + Duration::from_secs(10)));
        // Standing inside: after the few seconds.
        assert!(!arrival.update(&point, 120.0, 110.0, 0.2, start));
        assert!(!arrival.update(&point, 120.0, 110.0, 0.2, start + Duration::from_secs(2)));
        assert!(arrival.update(&point, 120.0, 110.0, 0.2, start + Duration::from_secs(5)));
        // Rolling on resets it.
        assert!(!arrival.update(&point, 120.0, 110.0, 3.0, start + Duration::from_secs(6)));
        assert!(!arrival.update(&point, 120.0, 110.0, 0.0, start + Duration::from_secs(7)));
    }

    #[test]
    fn a_tour_is_planned_as_phases() {
        let places = places();
        let trip = paris_tour(&places);
        let kinds: Vec<StopKind> = trip.stops.iter().map(|s| s.kind).collect();
        // Pick-up points first (home and the towns little out of the way), the depot last.
        assert_eq!(kinds[0], StopKind::Pickup);
        assert!(kinds.iter().filter(|k| **k == StopKind::Pickup).count() >= 2, "{kinds:?}");
        assert_eq!(*kinds.last().unwrap(), StopKind::Depot);
        assert!(kinds.contains(&StopKind::Hotel));
        assert!(kinds.contains(&StopKind::Excursion));
        assert!(kinds.iter().any(|k| matches!(k, StopKind::Coffee | StopKind::Meal)), "{kinds:?}");
        assert_eq!(
            kinds.iter().filter(|k| **k == StopKind::Pickup).count(),
            kinds.iter().filter(|k| **k == StopKind::Dropoff).count()
        );
        // Planned in order of time, the legs in order.
        assert!(trip.stops.windows(2).all(|w| w[0].at <= w[1].at + 1e-6), "{:?}", trip.stops.iter().map(|s| (s.kind, s.at)).collect::<Vec<_>>());
        let order = |leg: Leg| [Leg::Boarding, Leg::Outbound, Leg::Stay, Leg::Return, Leg::Unloading, Leg::ToDepot].iter().position(|l| *l == leg);
        assert!(trip.stops.windows(2).all(|w| order(w[0].leg) <= order(w[1].leg)));
        assert_eq!(trip.days, 5);

        // Driven: the phases one after another.
        let mut book = Book { catalogue: vec![trip], ..Book::default() };
        book.catalogue[0].booked = 40;
        assert!(book.start(7));
        let active = book.active.as_mut().unwrap();
        assert_eq!(active.trip.stops.iter().filter(|s| s.kind == StopKind::Pickup).map(|s| s.passengers).sum::<u32>(), 40);
        let mut phases = vec![active.phase()];
        while active.next_stop().is_some() {
            let at = active.next_stop().unwrap().at;
            active.reach(at);
            if active.resting_at.is_some() {
                active.leave_rest(at + 30.0);
            }
            if phases.last() != Some(&active.phase()) {
                phases.push(active.phase());
            }
        }
        assert_eq!(
            phases,
            [Phase::ToFirstPickup, Phase::Boarding, Phase::Outbound, Phase::Stay, Phase::Return, Phase::Unloading, Phase::ToDepot, Phase::Done]
        );
        assert_eq!(active.on_board, 0);
        assert!(active.score.pickups_late.iter().all(|late| *late == 0.0));
        assert_eq!(active.score.short_stops, 0);
    }

    #[test]
    fn a_long_way_has_an_overnight_on_the_way() {
        let mut places = places();
        places.push(place("city", "roma", 0.0, 70_000.0));
        places.push(place("city", "basel", 0.0, 35_000.0));
        places.push(place("hotel", "basel", 0.0, 35_100.0));
        let planning = planning(&places);
        let roma = places.iter().find(|p| p.city_id == "roma").unwrap();
        let trip = planning.trip(1, TripKind::Tour, "Italië", roma, 4, 0.0, 0.0, &mut Dice::new(2));
        let outbound_hotels = trip.stops.iter().filter(|s| s.leg == Leg::Outbound && s.kind == StopKind::Hotel).count();
        assert!(outbound_hotels >= 1);
        assert!(trip.days >= 4);
        assert!(trip.stops.last().unwrap().day <= trip.days, "{} > {}", trip.stops.last().unwrap().day, trip.days);
    }

    #[test]
    fn the_mood_from_punctuality_comfort_breaks_and_announcements() {
        let perfect = mood(&Score { pickups_late: vec![0.0, 2.0], good_stops: 3, ..Score::default() });
        assert_eq!(perfect.punctuality, 100.0);
        assert_eq!(perfect.comfort, 100.0);
        assert_eq!(perfect.breaks, 100.0);
        assert_eq!(perfect.total, 100.0);

        let late = mood(&Score { pickups_late: vec![13.0, 0.0], ..Score::default() });
        assert_eq!(late.punctuality, 80.0);

        let rough = mood(&Score { harsh_brakes: 5, harsh_corners: 5, harsh_accelerations: 4, ..Score::default() });
        assert_eq!(rough.comfort, 100.0 - 20.0 - 15.0 - 10.0);

        let tired = mood(&Score { overruns: 2, short_stops: 1, ..Score::default() });
        assert_eq!(tired.breaks, 80.0 - 40.0 - 10.0);
        assert!(tired.total < perfect.total);

        let base = mood(&Score::default());
        let told = mood(&Score { announcements: 2, ..Score::default() });
        assert_eq!(told.bonus, 5.0);
        assert_eq!(told.total, base.total + 5.0);
        assert_eq!(mood(&Score { announcements: 9, ..Score::default() }).bonus, 10.0);

        assert!(reputation_step(95.0) > 0.0);
        assert!(reputation_step(30.0) < 0.0);
    }

    #[test]
    fn harsh_braking_is_counted_once() {
        let start = Instant::now();
        let mut comfort = Comfort::default();
        let at = |ms: u64| start + Duration::from_millis(ms);
        assert_eq!(comfort.update(20.0, 0.0, at(0)), None);
        assert_eq!(comfort.update(19.9, 0.0, at(100)), None);
        // 0.5 m/s in 100 ms: 5 m/s².
        assert_eq!(comfort.update(19.4, 0.0, at(200)), Some(Harsh::Brake));
        assert_eq!(comfort.update(18.9, 0.0, at(300)), None);
        // Cornering hard: 20 m/s, a turn of 1/200 in 100 ms.
        let mut comfort = Comfort::default();
        comfort.update(20.0, 0.0, at(0));
        assert_eq!(comfort.update(20.0, 0.005, at(100)), Some(Harsh::Corner));
        // Over the heading's wrap: gentle.
        let mut comfort = Comfort::default();
        comfort.update(20.0, 0.9999, at(0));
        assert_eq!(comfort.update(20.0, 0.0001, at(100)), None);
    }

    fn day_trip(price: f64) -> Trip {
        Trip {
            id: 3,
            kind: TripKind::DayTrip,
            title: "Brugge".into(),
            destination: "Brugge".into(),
            days: 1,
            departure: 5.0 * MINUTES_PER_DAY,
            listed: 0.0,
            price,
            seats: 50,
            booked: 0,
            distance: 300.0,
            stops: Vec::new(),
        }
    }

    #[test]
    fn bookings_follow_reputation_price_season_and_time() {
        let fair = 29.0 + 300.0 * 0.07;
        let trip = day_trip(fair);
        let now = 2.0 * MINUTES_PER_DAY;
        let good = bookings(&trip, 90.0, now, 10);
        let poor = bookings(&trip, 10.0, now, 10);
        assert!(good > poor, "{good} vs {poor}");
        assert!(bookings(&day_trip(fair * 2.0), 90.0, now, 10) < good);
        assert!(bookings(&day_trip(fair * 0.7), 90.0, now, 10) >= good);
        assert!(bookings(&trip, 90.0, now, 7) > bookings(&trip, 90.0, now, 2));
        // Nearer the departure, more booked; never more than the seats.
        let early = bookings(&trip, 70.0, 0.0, 10);
        let late = bookings(&trip, 70.0, trip.departure, 10);
        assert!(late > early, "{late} vs {early}");
        assert!(bookings(&day_trip(1.0), 100.0, trip.departure, 7) <= 50);
        // A school trip: the whole class, once the reputation will do.
        let school = Trip { kind: TripKind::School, ..day_trip(4000.0) };
        assert_eq!(bookings(&school, 50.0, now, 3), 50);
        assert_eq!(bookings(&school, 10.0, now, 3), 0);
    }

    #[test]
    fn the_catalogue_is_made_from_the_map_and_kept_full() {
        let places = places();
        let planning = planning(&places);
        let mut book = Book::default();
        book.refresh(Some(&planning), 600.0, 7);
        assert_eq!(book.catalogue.len(), CATALOGUE);
        assert!(book.catalogue.iter().all(|t| t.departure > 600.0 && t.price > 0.0 && !t.stops.is_empty()));
        assert!(book.catalogue.iter().any(|t| t.kind == TripKind::Tour));
        // A trip past its departure goes; with bookings, the reputation suffers.
        let later = book.catalogue.iter().map(|t| t.departure).fold(0.0, f64::max) + 200.0;
        book.refresh(None, later, 7);
        assert!(book.catalogue.is_empty());
        assert!(book.reputation < 50.0);
    }

    #[test]
    fn money_income_less_fuel_tolls_ferries_hotel_and_wage() {
        let places = places();
        let mut trip = paris_tour(&places);
        trip.booked = 40;
        let meter = Meter { litres: 200.0, km: 1000.0, ferries: 1 };
        let bill = costs(&trip, &meter);
        assert_eq!(bill.fuel, 200.0 * FUEL_PER_LITRE);
        assert_eq!(bill.tolls, 1000.0 * TOLL_PER_KM);
        assert_eq!(bill.ferries, FERRY);
        assert_eq!(bill.hotel, 4.0 * 41.0 * HOTEL_PER_NIGHT);
        assert_eq!(bill.wage, 5.0 * WAGE_PER_DAY);

        let mut book = Book { catalogue: vec![trip.clone()], ..Book::default() };
        assert!(book.start(trip.id));
        let active = book.active.as_mut().unwrap();
        active.meter = meter;
        active.score.good_stops = 3;
        let finished = book.settle(10_000.0, false).unwrap();
        assert_eq!(finished.income, trip.price * 40.0);
        assert!((finished.profit - (finished.income - bill.total())).abs() < 1e-6);
        assert!((book.balance - (25_000.0 + finished.profit)).abs() < 1e-6);
        assert!(book.reputation > 50.0);
        assert_eq!(book.history.len(), 1);

        // Called off: no income, the reputation down.
        let mut book = Book { catalogue: vec![trip.clone()], ..Book::default() };
        book.start(trip.id);
        let finished = book.settle(trip.departure + 60.0, true).unwrap();
        assert_eq!(finished.income, 0.0);
        assert_eq!(finished.costs.hotel, 0.0);
        assert_eq!(book.reputation, 45.0);
    }

    #[test]
    fn announcements_asked_at_the_right_moments() {
        let places = places();
        let mut trip = paris_tour(&places);
        trip.booked = 30;
        let mut book = Book { catalogue: vec![trip.clone()], ..Book::default() };
        book.start(trip.id);
        let active = book.active.as_mut().unwrap();
        let at = active.next_stop().unwrap().at;
        let reached = active.reach(at).unwrap();
        assert_eq!(prompt_due(active, None, Some(&reached), at).unwrap().kind, "welcome");
        let reached = active.reach(at + 30.0).unwrap();
        let prompt = prompt_due(active, None, Some(&reached), at + 30.0).unwrap();
        assert!(matches!(prompt.kind.as_str(), "nextPickup" | "departure"));
        while !matches!(active.next_stop().unwrap().kind, StopKind::Coffee | StopKind::Meal) {
            active.reach(0.0);
        }
        assert!(prompt_due(active, Some(30.0), None, 0.0).is_none());
        assert!(matches!(prompt_due(active, Some(8.0), None, 0.0).unwrap().kind.as_str(), "coffee" | "meal"));
    }

    #[test]
    fn the_book_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("dashly-touring-{}", std::process::id()));
        let places = places();
        let mut book = Book::default();
        book.refresh(Some(&planning(&places)), 0.0, 5);
        let id = book.catalogue[0].id;
        book.start(id);
        book.save(&dir).unwrap();
        // The same (as JSON: a float's last digit may round).
        let json = |book: &Book| serde_json::to_value(book).unwrap();
        let loaded = Book::load(&dir);
        assert_eq!(loaded.catalogue.len(), book.catalogue.len());
        assert_eq!(loaded.active.as_ref().map(|a| a.trip.id), Some(id));
        assert_eq!(json(&loaded)["active"]["trip"]["stops"].as_array().map(Vec::len), json(&book)["active"]["trip"]["stops"].as_array().map(Vec::len));
        let _ = std::fs::remove_dir_all(dir);
    }
}
