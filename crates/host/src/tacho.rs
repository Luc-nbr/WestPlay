//! Euro-Hub's tachograph: a digital driving-time recorder of its own (no maker's look), kept live from the game's
//! telemetry and checked against the EU's driving and rest rules (Regulation (EC) No 561/2006):
//!
//! - at most 4.5 hours of driving, then a break of 45 minutes (or 15 and then 30);
//! - at most 9 hours of driving between two daily rests, 10 hours twice a week;
//! - a daily rest of 11 hours (or 9, reduced, at most three times a week; 3 + 9 split counts as regular),
//!   started within 24 hours of the end of the last one;
//! - at most 56 hours of driving a week (Monday 00:00 to Sunday 24:00), 90 in two weeks in a row.
//!
//! Time is kept on the tachograph's own clock, in minutes: the game's clock ([`Clock::Game`], the default: the
//! game's own day, its sleep counting as the daily rest, as its fatigue does) or the PC's ([`Clock::Real`]: real
//! hours, Euro-Hub closed counting as rest). Each clock keeps its own book (`tachograph/game.json`,
//! `tachograph/real.json` in Euro-Hub's data folder): their times can't be mixed.
//!
//! The book is a list of activities (what, from, to); everything else (what's left, what was broken) is
//! worked out from it again ([`evaluate`]), so it's the same after a restart.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

/// Set by the companion's "Clear the log": the host empties the book on its next tick.
pub static RESET: AtomicBool = AtomicBool::new(false);

pub const MINUTES_PER_DAY: f64 = 1440.0;
pub const MINUTES_PER_WEEK: f64 = 7.0 * MINUTES_PER_DAY;

/// The rules, in minutes.
pub const CONTINUOUS_DRIVING: f64 = 270.0;
pub const BREAK_FULL: f64 = 45.0;
pub const BREAK_FIRST: f64 = 15.0;
pub const BREAK_SECOND: f64 = 30.0;
pub const DAILY_DRIVING: f64 = 540.0;
pub const DAILY_DRIVING_EXTENDED: f64 = 600.0;
pub const EXTENDED_DAYS_PER_WEEK: u32 = 2;
pub const DAILY_REST: f64 = 660.0;
pub const DAILY_REST_REDUCED: f64 = 540.0;
pub const DAILY_REST_SPLIT_FIRST: f64 = 180.0;
pub const REDUCED_RESTS_PER_WEEK: u32 = 3;
pub const REST_WITHIN: f64 = 1440.0;
pub const WEEKLY_DRIVING: f64 = 3360.0;
pub const FORTNIGHT_DRIVING: f64 = 5400.0;

/// How much history is kept: four weeks (two are needed for the rules, the rest for the log).
const KEEP: f64 = 29.0 * MINUTES_PER_DAY;
const KEEP_VIOLATIONS: usize = 200;

/// Which clock the tachograph runs on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Clock {
    #[default]
    Game,
    Real,
}

impl Clock {
    fn file(self) -> &'static str {
        match self {
            Clock::Game => "game.json",
            Clock::Real => "real.json",
        }
    }

    /// A step of the clock this long in one tick isn't driving or working: the game skipped time (sleep, a
    /// ferry, a train) or Euro-Hub didn't run: rest.
    fn jump(self) -> f64 {
        match self {
            Clock::Game => 10.0,
            Clock::Real => 2.0,
        }
    }

    /// A step larger than this is another save or profile (game) and isn't counted at all. Real time always is.
    fn unrelated(self) -> f64 {
        match self {
            Clock::Game => 3.0 * MINUTES_PER_DAY,
            Clock::Real => f64::INFINITY,
        }
    }
}

/// What the driver does, as a tachograph's four symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Activity {
    Driving,
    Work,
    Available,
    Rest,
}

/// One activity, from and to (tachograph minutes).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub activity: Activity,
    pub start: f64,
    pub end: f64,
}

impl Segment {
    pub fn minutes(&self) -> f64 {
        self.end - self.start
    }
}

/// A rule broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ViolationKind {
    /// More than 4.5 hours of driving without the break.
    ContinuousDriving,
    /// More than 9 hours (10 on an extended day) between daily rests.
    DailyDriving,
    /// No daily rest of at least 9 hours started in time (within 24 hours).
    DailyRestLate,
    /// A fourth reduced daily rest in a week.
    ReducedRests,
    /// More than 56 hours of driving in a week.
    WeeklyDriving,
    /// More than 90 hours in two weeks in a row.
    FortnightDriving,
    /// An event ("voorval"): faster than the truck's 90 km/h for more than 60 seconds, as a tachograph records it.
    Overspeed,
}

/// A rule broken: what, which period it belongs to (what tells it apart, so it's recorded once), when it
/// happened, and how far over (minutes).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub kind: ViolationKind,
    pub period: f64,
    pub at: f64,
    pub over: f64,
}

impl Violation {
    fn same(&self, other: &Violation) -> bool {
        self.kind == other.kind && (self.period - other.period).abs() < 0.5
    }
}

/// The book: the clock's time now, the raw time last seen, the activities and the violations recorded.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Book {
    pub now: f64,
    pub last_raw: Option<f64>,
    /// Kilometres driven while it recorded (the display's odometer).
    pub distance: f64,
    pub segments: Vec<Segment>,
    pub violations: Vec<Violation>,
}

impl Book {
    /// The clock read again (`raw`: game minutes, or minutes since 1970), the driver doing `activity` since the
    /// last tick. Returns whether the clock moved.
    pub fn tick(&mut self, clock: Clock, raw: f64, activity: Activity) -> bool {
        let Some(last) = self.last_raw.replace(raw) else {
            if self.segments.is_empty() {
                self.now = raw;
            }
            return false;
        };
        let step = raw - last;
        if step <= 0.0 || step > clock.unrelated() {
            // An older save loaded, or another profile: the clock carries on from where it was.
            return false;
        }
        let activity = if step > clock.jump() { Activity::Rest } else { activity };
        let start = self.now;
        self.now += step;
        self.add(activity, start, self.now);
        true
    }

    fn add(&mut self, activity: Activity, start: f64, end: f64) {
        match self.segments.last_mut() {
            Some(last) if last.activity == activity && (last.end - start).abs() < 1e-6 => last.end = end,
            _ => self.segments.push(Segment { activity, start, end }),
        }
        let cutoff = self.now - KEEP;
        let old = self.segments.iter().take_while(|segment| segment.end < cutoff).count();
        self.segments.drain(..old);
    }

    /// Records the violations not recorded yet; returns them.
    pub fn record(&mut self, found: &[Violation]) -> Vec<Violation> {
        let mut new = Vec::new();
        for violation in found {
            match self.violations.iter_mut().find(|known| known.same(violation)) {
                // How far over grows while it goes on.
                Some(known) => known.over = known.over.max(violation.over),
                None => new.push(*violation),
            }
        }
        self.violations.extend(&new);
        let extra = self.violations.len().saturating_sub(KEEP_VIOLATIONS);
        self.violations.drain(..extra);
        new
    }

    /// The activity now (`None` before the first).
    pub fn activity(&self) -> Option<Activity> {
        self.segments.last().map(|segment| segment.activity)
    }

    pub fn path(dir: &Path, clock: Clock) -> PathBuf {
        dir.join("tachograph").join(clock.file())
    }

    /// The book kept for `clock`, or a new one (an unreadable one set aside as `.bad`).
    pub fn load(dir: &Path, clock: Clock) -> Self {
        let path = Self::path(dir, clock);
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                let _ = std::fs::rename(&path, path.with_extension("json.bad"));
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Kept on disk (written aside first, then put in place).
    pub fn save(&self, dir: &Path, clock: Clock) -> std::io::Result<()> {
        let path = Self::path(dir, clock);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let aside = path.with_extension("json.new");
        std::fs::write(&aside, serde_json::to_vec(self).map_err(std::io::Error::other)?)?;
        std::fs::rename(aside, path)
    }
}

/// Which limit comes first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Limit {
    #[default]
    Break,
    Day,
    Week,
    Fortnight,
}

/// Where the driver stands now (minutes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub activity: Option<Activity>,
    /// How long the current activity has gone on.
    pub activity_for: f64,
    /// Driving since the last qualifying break, and what's left of the 4.5 hours.
    pub driving_since_break: f64,
    pub break_left: f64,
    /// The break still owed (45, or 30 after a first 15), and how much of it the current rest has given.
    pub break_needed: f64,
    pub break_taken: f64,
    /// Driving since the last daily rest, its limit (9 or 10 hours), and what's left.
    pub day_driving: f64,
    pub day_limit: f64,
    pub day_left: f64,
    pub extended_days_used: u32,
    pub reduced_rests_used: u32,
    /// Minutes until the daily rest has to start at the latest (negative: late), counted from the end of the
    /// last daily rest.
    pub rest_due_in: f64,
    /// The daily rest taken so far, while resting.
    pub rest_taken: f64,
    pub week_driving: f64,
    pub week_left: f64,
    pub fortnight_driving: f64,
    pub fortnight_left: f64,
    /// The first limit reached, and how long until it is (negative: over).
    pub next_limit: Limit,
    pub next_left: f64,
}

/// Where the week starts: game minute 0 is a Monday, 00:00; 1970-01-01 a Thursday (`local_offset`: the PC's
/// minutes ahead of UTC).
pub fn week_origin(clock: Clock, local_offset: f64) -> f64 {
    match clock {
        Clock::Game => 0.0,
        Clock::Real => 4.0 * MINUTES_PER_DAY - local_offset,
    }
}

fn week_of(t: f64, origin: f64) -> i64 {
    ((t - origin) / MINUTES_PER_WEEK).floor() as i64
}

/// A counter of minutes that records the moment it went past its limit.
fn crossing(count: f64, add: f64, limit: f64, start: f64) -> Option<(f64, f64)> {
    (count <= limit && count + add > limit).then_some((start + (limit - count), count + add - limit))
}

/// Everything worked out from the book at `now`: where the driver stands, and every rule broken in it.
pub fn evaluate(book: &Book, origin: f64) -> (Summary, Vec<Violation>) {
    let now = book.now;
    let mut violations = Vec::new();
    let Some(first) = book.segments.first() else {
        let mut summary = Summary {
            break_left: CONTINUOUS_DRIVING,
            break_needed: BREAK_FULL,
            day_limit: DAILY_DRIVING_EXTENDED,
            day_left: DAILY_DRIVING_EXTENDED,
            rest_due_in: REST_WITHIN - DAILY_REST_REDUCED,
            week_left: WEEKLY_DRIVING,
            fortnight_left: FORTNIGHT_DRIVING,
            ..Summary::default()
        };
        summary.next_left = CONTINUOUS_DRIVING;
        return (summary, violations);
    };

    // The 4.5 hours: driving since the last qualifying break, when that driving began, and a first 15 taken.
    let mut driven = 0.0;
    let mut driven_since = first.start;
    let mut had_first = false;
    // The day: since the end of the last daily rest.
    let mut period_start = first.start;
    let mut period_driving = 0.0;
    let mut split_first = false;
    let mut late_counted = false;
    // The weeks: driving, extended days, reduced rests, by week.
    let mut week_driving: std::collections::HashMap<i64, f64> = Default::default();
    let mut extended: std::collections::HashMap<i64, u32> = Default::default();
    let mut reduced: std::collections::HashMap<i64, u32> = Default::default();

    let last = book.segments.len() - 1;
    let mut rest_taken = 0.0;
    // A first 15 taken before the rest going on now (which may yet become the whole 45).
    let mut had_first_before = false;
    for (i, segment) in book.segments.iter().enumerate() {
        let open = i == last;
        let minutes = segment.minutes();
        match segment.activity {
            Activity::Driving => {
                if driven == 0.0 {
                    driven_since = segment.start;
                }
                if let Some((at, over)) = crossing(driven, minutes, CONTINUOUS_DRIVING, segment.start) {
                    violations.push(Violation { kind: ViolationKind::ContinuousDriving, period: driven_since, at, over });
                } else if driven > CONTINUOUS_DRIVING
                    && let Some(known) = violations
                        .iter_mut()
                        .rev()
                        .find(|v| v.kind == ViolationKind::ContinuousDriving && v.period == driven_since)
                {
                    known.over += minutes;
                }
                driven += minutes;

                let week = week_of(period_start, origin);
                let limit = if extended.get(&week).copied().unwrap_or(0) < EXTENDED_DAYS_PER_WEEK {
                    DAILY_DRIVING_EXTENDED
                } else {
                    DAILY_DRIVING
                };
                if let Some((at, over)) = crossing(period_driving, minutes, limit, segment.start) {
                    violations.push(Violation { kind: ViolationKind::DailyDriving, period: period_start, at, over });
                }
                period_driving += minutes;

                // Driving split at the weeks' starts.
                let mut start = segment.start;
                while start < segment.end {
                    let week = week_of(start, origin);
                    let week_end = origin + (week + 1) as f64 * MINUTES_PER_WEEK;
                    let end = segment.end.min(week_end);
                    let add = end - start;
                    let this = week_driving.get(&week).copied().unwrap_or(0.0);
                    let before = week_driving.get(&(week - 1)).copied().unwrap_or(0.0);
                    let week_start = origin + week as f64 * MINUTES_PER_WEEK;
                    if let Some((at, over)) = crossing(this, add, WEEKLY_DRIVING, start) {
                        violations.push(Violation { kind: ViolationKind::WeeklyDriving, period: week_start, at, over });
                    }
                    if let Some((at, over)) = crossing(before + this, add, FORTNIGHT_DRIVING, start) {
                        violations.push(Violation {
                            kind: ViolationKind::FortnightDriving,
                            period: week_start,
                            at,
                            over,
                        });
                    }
                    week_driving.insert(week, this + add);
                    start = end;
                }
            }
            Activity::Rest => {
                if open {
                    had_first_before = had_first;
                }
                // Breaks: 45 at once, or 15 and later 30.
                if minutes >= BREAK_FULL || (had_first && minutes >= BREAK_SECOND) {
                    driven = 0.0;
                    had_first = false;
                } else if minutes >= BREAK_FIRST && driven > 0.0 {
                    had_first = true;
                }
                if open {
                    rest_taken = minutes;
                }
                // The daily rest.
                if minutes >= DAILY_REST_REDUCED {
                    if segment.start > period_start + REST_WITHIN - DAILY_REST_REDUCED && !late_counted {
                        violations.push(Violation {
                            kind: ViolationKind::DailyRestLate,
                            period: period_start,
                            at: segment.start,
                            over: segment.start - (period_start + REST_WITHIN - DAILY_REST_REDUCED),
                        });
                    }
                    let week = week_of(period_start, origin);
                    if period_driving > DAILY_DRIVING {
                        *extended.entry(week).or_default() += 1;
                    }
                    // A reduced rest is counted once it's over (an open one may still reach 11 hours).
                    if !open && minutes < DAILY_REST && !split_first {
                        let week = week_of(segment.start, origin);
                        let count = reduced.entry(week).or_default();
                        *count += 1;
                        if *count > REDUCED_RESTS_PER_WEEK {
                            violations.push(Violation {
                                kind: ViolationKind::ReducedRests,
                                period: segment.start,
                                at: segment.end,
                                over: DAILY_REST - minutes,
                            });
                        }
                    }
                    period_start = segment.end;
                    period_driving = 0.0;
                    split_first = false;
                    late_counted = false;
                } else if minutes >= DAILY_REST_SPLIT_FIRST {
                    split_first = true;
                }
            }
            Activity::Work | Activity::Available => {}
        }
        // No daily rest begun in time: once 24 hours are up with none, it's late.
        if !late_counted && segment.end > period_start + REST_WITHIN {
            late_counted = true;
            let resting_in_time = segment.activity == Activity::Rest
                && segment.start <= period_start + REST_WITHIN - DAILY_REST_REDUCED;
            if !resting_in_time {
                violations.push(Violation {
                    kind: ViolationKind::DailyRestLate,
                    period: period_start,
                    at: period_start + REST_WITHIN,
                    over: segment.end - (period_start + REST_WITHIN),
                });
            }
        }
    }

    let current = book.segments[last];
    let week = week_of(now, origin);
    let this_week = week_driving.get(&week).copied().unwrap_or(0.0);
    let fortnight = this_week + week_driving.get(&(week - 1)).copied().unwrap_or(0.0);
    let period_week = week_of(period_start, origin);
    let extended_used = extended.get(&period_week).copied().unwrap_or(0);
    let day_limit = if extended_used < EXTENDED_DAYS_PER_WEEK { DAILY_DRIVING_EXTENDED } else { DAILY_DRIVING };
    let reduced_used = reduced.get(&week_of(now, origin)).copied().unwrap_or(0);
    let rest_needed = if reduced_used < REDUCED_RESTS_PER_WEEK { DAILY_REST_REDUCED } else { DAILY_REST };

    let resting = current.activity == Activity::Rest;
    let first_taken = if resting { had_first_before } else { had_first };
    let mut summary = Summary {
        activity: Some(current.activity),
        activity_for: current.minutes(),
        driving_since_break: driven,
        break_left: CONTINUOUS_DRIVING - driven,
        break_needed: if first_taken { BREAK_SECOND } else { BREAK_FULL },
        break_taken: if resting { rest_taken } else { 0.0 },
        day_driving: period_driving,
        day_limit,
        day_left: day_limit - period_driving,
        extended_days_used: extended_used + u32::from(period_driving > DAILY_DRIVING),
        reduced_rests_used: reduced_used,
        rest_due_in: period_start + REST_WITHIN - rest_needed - now,
        rest_taken: if resting { rest_taken } else { 0.0 },
        week_driving: this_week,
        week_left: WEEKLY_DRIVING - this_week,
        fortnight_driving: fortnight,
        fortnight_left: FORTNIGHT_DRIVING - fortnight,
        ..Summary::default()
    };
    let (limit, left) = [
        (Limit::Break, summary.break_left),
        (Limit::Day, summary.day_left),
        (Limit::Week, summary.week_left),
        (Limit::Fortnight, summary.fortnight_left),
    ]
    .into_iter()
    .fold((Limit::Break, f64::INFINITY), |best, (limit, left)| if left < best.1 { (limit, left) } else { best });
    summary.next_limit = limit;
    summary.next_left = left;
    (summary, violations)
}

/// What the driver does, from the truck: driving while it moves, working while it stands with the engine
/// running, resting with the engine off. A change holds for [`Detector::HOLD`] before it counts, so a moment
/// at a standstill isn't a break.
#[derive(Debug, Default)]
pub struct Detector {
    current: Option<Activity>,
    candidate: Option<(Activity, std::time::Instant)>,
}

impl Detector {
    pub const HOLD: std::time::Duration = std::time::Duration::from_secs(3);
    /// Moving: faster than this (m/s, about 2 km/h).
    pub const MOVING: f32 = 0.6;

    pub fn seen(speed: f32, engine: bool) -> Activity {
        if speed.abs() > Self::MOVING {
            Activity::Driving
        } else if engine {
            Activity::Work
        } else {
            Activity::Rest
        }
    }

    pub fn update(&mut self, seen: Activity, now: std::time::Instant) -> Activity {
        match self.current {
            None => self.current = Some(seen),
            Some(current) if current == seen => self.candidate = None,
            Some(_) => match self.candidate {
                Some((candidate, since)) if candidate == seen => {
                    if now.duration_since(since) >= Self::HOLD {
                        self.current = Some(seen);
                        self.candidate = None;
                    }
                }
                _ => self.candidate = Some((seen, now)),
            },
        }
        self.current.unwrap_or(seen)
    }
}

/// What the companion shows of the log: activities and violations, newest first.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LogEntry {
    Activity { activity: Activity, start: f64, end: f64 },
    Violation { kind: ViolationKind, at: f64, over: f64 },
}

pub fn log(book: &Book, count: usize) -> Vec<LogEntry> {
    let mut entries: Vec<LogEntry> = book
        .segments
        .iter()
        .enumerate()
        // Moments (a stop at the lights) left out of the log; the rules still count them.
        .filter(|(i, s)| s.minutes() >= 1.0 || i + 1 == book.segments.len())
        .map(|(_, s)| s)
        .map(|s| LogEntry::Activity { activity: s.activity, start: s.start, end: s.end })
        .chain(book.violations.iter().map(|v| LogEntry::Violation { kind: v.kind, at: v.at, over: v.over }))
        .collect();
    let time = |entry: &LogEntry| match entry {
        LogEntry::Activity { start, .. } => *start,
        LogEntry::Violation { at, .. } => *at,
    };
    entries.sort_by(|a, b| time(b).total_cmp(&time(a)));
    entries.truncate(count);
    entries
}

/// The tachograph as the companion sees it.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TachoStatus {
    pub enabled: bool,
    pub clock: Clock,
    /// The clock's time now (tachograph minutes: game minutes, or minutes since 1970).
    pub now: f64,
    pub summary: Summary,
    pub log: Vec<LogEntry>,
    /// Live from the game: the truck's speed (km/h), the kilometres recorded, the truck (brand and model), the game's
    /// clock (game minutes; `None`: no game), and whether the driver's card is in its slot.
    pub speed: f64,
    pub distance: f64,
    pub truck: Option<String>,
    pub game_time: Option<f64>,
    pub card: bool,
    /// The game's profile played, for the driver card and its stats (`None`: not read yet).
    pub profile: Option<Profile>,
}

/// The game's profile, as the driver card and the Tachograph page show it: its name (the card's driver), its
/// company, a card number of its own (from the profile's folder, the same every time), and from its save the
/// kilometres driven, the deliveries made, the experience and the money in the bank.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub company: String,
    pub card_number: String,
    pub distance: u64,
    pub deliveries: u64,
    pub experience: u64,
    pub money: i64,
}

impl Profile {
    /// From a profile's folder name (`id`), its `profile.sii` and its save's units.
    pub fn read(id: &str, profile: &[westplay_mapdata::sii::Unit], save: &[westplay_mapdata::sii::Unit]) -> Option<Self> {
        let user = profile.iter().find(|unit| unit.class == "user_profile")?;
        let number = |unit: Option<&westplay_mapdata::sii::Unit>, key: &str| {
            unit.and_then(|unit| unit.get(key)).and_then(|value| value.parse::<f64>().ok()).unwrap_or(0.0)
        };
        let of = |class: &str| save.iter().find(|unit| unit.class == class);
        Some(Self {
            name: user.get("profile_name").unwrap_or_default().trim_matches('"').to_owned(),
            company: user.get("company_name").unwrap_or_default().trim_matches('"').to_owned(),
            card_number: card_number(id),
            distance: number(Some(user), "cached_distance").max(0.0) as u64,
            deliveries: number(of("delivery_log"), "cached_jobs_count").max(0.0) as u64,
            experience: number(of("economy"), "experience_points").max(0.0) as u64,
            money: number(of("bank"), "money_account") as i64,
        })
    }
}

/// A driver card's number for a profile: NL and 14 digits, from its folder name, so each profile keeps its own.
pub fn card_number(id: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    format!("NL{:014}", hash % 100_000_000_000_000)
}

/// A warning to give: a limit `minutes` away (15, 5), or reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warning {
    Soon(Limit, u32),
    Reached(Limit),
}

/// Warnings as the time left falls past 15 and 5 minutes and 0, while driving.
#[derive(Debug, Default)]
pub struct Warner {
    last: Option<f64>,
}

impl Warner {
    pub const AHEAD: [u32; 2] = [15, 5];

    pub fn update(&mut self, summary: &Summary) -> Option<Warning> {
        let left = summary.next_left;
        let before = self.last.replace(left)?;
        if summary.activity != Some(Activity::Driving) {
            return None;
        }
        if before > 0.0 && left <= 0.0 {
            return Some(Warning::Reached(summary.next_limit));
        }
        Self::AHEAD
            .into_iter()
            .find(|&ahead| before > ahead as f64 && left <= ahead as f64)
            .map(|ahead| Warning::Soon(summary.next_limit, ahead))
    }
}

/// The tachograph as the host runs it: the book of the clock chosen, ticked about once a second from the game's
/// telemetry, kept on disk, and what it tells the companion and the cab.
pub struct Tachograph {
    clock: Clock,
    book: Book,
    detector: Detector,
    warner: Warner,
    last_tick: std::time::Instant,
    saved: std::time::Instant,
    saved_activity: Option<Activity>,
    /// Warnings given (the cab's banner flashes with each), the violation last found and when, the cab's last
    /// message and when it was sent.
    pings: u32,
    violation: Option<(ViolationKind, std::time::Instant)>,
    sent: Option<(String, std::time::Instant)>,
    /// Faster than [`OVERSPEED_KMH`] since: the book's minute and the moment.
    overspeed: Option<(f64, std::time::Instant)>,
    /// The game's profile, as its save last said.
    profile: Option<Profile>,
}

/// A truck's set top speed (EU speed limiters), and how long past it is an event.
const OVERSPEED_KMH: f32 = 90.0;
const OVERSPEED_AFTER: std::time::Duration = std::time::Duration::from_secs(60);

/// How long the cab shows a violation just found.
const VIOLATION_SHOWN: std::time::Duration = std::time::Duration::from_secs(20);

impl Tachograph {
    pub fn start(dir: &Path, clock: Clock) -> Self {
        let now = std::time::Instant::now();
        Self {
            clock,
            book: Book::load(dir, clock),
            detector: Detector::default(),
            warner: Warner::default(),
            last_tick: now - std::time::Duration::from_secs(5),
            saved: now,
            saved_activity: None,
            pings: 0,
            violation: None,
            sent: None,
            overspeed: None,
            profile: None,
        }
    }

    /// The game's profile, read again from a newer save.
    pub fn profile(&mut self, profile: Option<Profile>) {
        if profile.is_some() {
            self.profile = profile;
        }
    }

    /// Whether a tick is due (about once a second).
    pub fn due(&self) -> bool {
        self.last_tick.elapsed() >= std::time::Duration::from_secs(1)
    }

    pub fn save(&mut self, dir: &Path) {
        self.saved = std::time::Instant::now();
        self.saved_activity = self.book.activity();
        if let Err(e) = self.book.save(dir, self.clock) {
            crate::log!("[tacho] couldn't keep the book: {e}");
        }
    }

    /// A tick: the game's telemetry now (`None`: no game). Returns the companion's status (`None`: off) and a
    /// message for the cab's page, when there's news for it.
    pub fn tick(
        &mut self,
        dir: &Path,
        settings: &crate::settings::TachoSettings,
        telemetry: Option<&westplay_shared::Telemetry>,
        truck: Option<&westplay_shared::TruckInfo>,
    ) -> (Option<TachoStatus>, Option<String>) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_tick).as_secs_f64().min(5.0);
        self.last_tick = now;
        if settings.clock != self.clock {
            self.save(dir);
            let profile = self.profile.take();
            *self = Self { saved: now, profile, ..Self::start(dir, settings.clock) };
        }
        if RESET.swap(false, std::sync::atomic::Ordering::AcqRel) {
            self.book = Book::default();
            self.violation = None;
            self.save(dir);
            crate::log!("[tacho] the log cleared");
        }
        if !settings.enabled {
            return (None, self.message(None, settings));
        }
        let playing = telemetry.filter(|t| !t.paused);
        if let Some(t) = playing {
            self.book.distance += f64::from(t.speed.abs()) * elapsed / 1000.0;
        }
        let seen = playing.map_or(Activity::Rest, |t| Detector::seen(t.speed, t.engine_enabled));
        let activity = self.detector.update(seen, now);
        let raw = match self.clock {
            // The game's clock stands still while it's paused or closed: nothing to count.
            Clock::Game => playing.map(|t| f64::from(t.game_time)),
            Clock::Real => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|since| since.as_secs_f64() / 60.0),
        };
        if let Some(raw) = raw {
            self.book.tick(self.clock, raw, activity);
        }
        let (summary, mut found) = evaluate(&self.book, week_origin(self.clock, local_offset()));
        let fast = playing.is_some_and(|t| t.speed.abs() * 3.6 > OVERSPEED_KMH);
        match (fast, self.overspeed) {
            (false, _) => self.overspeed = None,
            (true, None) => self.overspeed = Some((self.book.now, now)),
            (true, Some((start, since))) if now.duration_since(since) >= OVERSPEED_AFTER => {
                found.push(Violation {
                    kind: ViolationKind::Overspeed,
                    period: start,
                    at: start,
                    over: (self.book.now - start).max(1.0),
                });
            }
            (true, Some(_)) => {}
        }
        for violation in self.book.record(&found) {
            crate::log!("[tacho] broken: {:?}, {:.0} min over", violation.kind, violation.over);
            self.violation = Some((violation.kind, now));
            self.save(dir);
        }
        if let Some(warning) = self.warner.update(&summary) {
            crate::log!("[tacho] {warning:?}");
            self.pings += 1;
        }
        if self.book.activity() != self.saved_activity || self.saved.elapsed() >= std::time::Duration::from_secs(30) {
            self.save(dir);
        }
        let status = TachoStatus {
            enabled: true,
            clock: self.clock,
            now: self.book.now,
            summary,
            log: log(&self.book, 120),
            speed: playing.map_or(0.0, |t| (f64::from(t.speed.abs()) * 3.6).round()),
            distance: self.book.distance,
            truck: truck.filter(|t| !t.brand.as_str().is_empty()).map(|t| {
                format!("{} {}", t.brand.as_str(), t.name.as_str()).trim().to_owned()
            }),
            game_time: telemetry.map(|t| f64::from(t.game_time)),
            card: settings.card,
            profile: self.profile.clone(),
        };
        (Some(status), self.message(Some(&summary), settings))
    }

    /// The cab's banner: a limit near or reached, a daily rest due, a violation just found; sent when it
    /// changes, and every few seconds (a page loaded again hears it).
    fn message(&mut self, summary: Option<&Summary>, settings: &crate::settings::TachoSettings) -> Option<String> {
        let now = std::time::Instant::now();
        let shown = settings.enabled && settings.cab_warnings;
        let alert = summary.filter(|_| shown).and_then(|summary| {
            let working = matches!(summary.activity, Some(Activity::Driving | Activity::Work));
            let limit = serde_json::to_value(summary.next_limit).ok()?;
            if summary.next_left <= 0.0 && working {
                Some(serde_json::json!({ "tone": "over", "limit": limit, "minutes": 0 }))
            } else if summary.next_left <= 15.0 && summary.activity == Some(Activity::Driving) {
                Some(serde_json::json!({ "tone": "soon", "limit": limit, "minutes": summary.next_left.ceil() }))
            } else if summary.rest_due_in <= 15.0 && working {
                Some(serde_json::json!({
                    "tone": if summary.rest_due_in <= 0.0 { "over" } else { "soon" },
                    "limit": "rest",
                    "minutes": summary.rest_due_in.max(0.0).ceil(),
                }))
            } else if !settings.card && summary.activity == Some(Activity::Driving) {
                // Driving without the driver's card in its slot.
                Some(serde_json::json!({ "tone": "over", "limit": "card", "minutes": 0 }))
            } else {
                None
            }
        });
        let violation = self
            .violation
            .filter(|(_, at)| shown && at.elapsed() < VIOLATION_SHOWN)
            .and_then(|(kind, _)| serde_json::to_value(kind).ok());
        let message =
            serde_json::json!({ "type": "tacho", "alert": alert, "violation": violation, "ping": self.pings }).to_string();
        let resend = self.sent.as_ref().is_none_or(|(sent, at)| *sent != message || at.elapsed().as_secs() >= 5);
        resend.then(|| {
            self.sent = Some((message.clone(), now));
            message
        })
    }
}

/// The PC's minutes ahead of UTC now (summer time counted).
fn local_offset() -> f64 {
    use windows::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};
    let mut zone = TIME_ZONE_INFORMATION::default();
    // SAFETY: fills the struct it's given.
    let kind = unsafe { GetTimeZoneInformation(&mut zone) };
    // 2: summer time now (TIME_ZONE_ID_DAYLIGHT).
    let bias = zone.Bias + if kind == 2 { zone.DaylightBias } else { zone.StandardBias };
    -f64::from(bias)
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: f64 = 60.0;

    /// A book from activities and their minutes, from game minute 0 (a Monday).
    fn book(plan: &[(Activity, f64)]) -> Book {
        let mut book = Book { last_raw: Some(0.0), ..Book::default() };
        let mut raw = 0.0;
        for &(activity, minutes) in plan {
            // Ticks of a minute, as the host's would be (shorter, in fact).
            let mut left = minutes;
            while left > 0.0 {
                let step = left.min(1.0);
                raw += step;
                book.tick(Clock::Game, raw, activity);
                left -= step;
            }
        }
        book
    }

    fn kinds(violations: &[Violation]) -> Vec<ViolationKind> {
        violations.iter().map(|v| v.kind).collect()
    }

    use Activity::*;

    #[test]
    fn four_and_a_half_hours_then_a_break() {
        let (summary, violations) = evaluate(&book(&[(Driving, 4.0 * H)]), 0.0);
        assert_eq!(summary.break_left, 30.0);
        assert_eq!(summary.next_limit, Limit::Break);
        assert!(violations.is_empty());

        let (summary, violations) = evaluate(&book(&[(Driving, 4.5 * H), (Rest, 45.0), (Driving, 60.0)]), 0.0);
        assert_eq!(summary.driving_since_break, 60.0);
        assert!(violations.is_empty());

        let (_, violations) = evaluate(&book(&[(Driving, 5.0 * H)]), 0.0);
        assert_eq!(kinds(&violations), [ViolationKind::ContinuousDriving]);
        assert_eq!(violations[0].at, 270.0);
        assert_eq!(violations[0].over, 30.0);
    }

    #[test]
    fn a_break_split_fifteen_then_thirty() {
        let plan = [(Driving, 2.0 * H), (Rest, 15.0), (Driving, 2.0 * H), (Rest, 30.0), (Driving, 4.0 * H)];
        let (summary, violations) = evaluate(&book(&plan), 0.0);
        assert!(violations.is_empty(), "{violations:?}");
        assert_eq!(summary.driving_since_break, 4.0 * H);

        // Thirty first and fifteen after doesn't count: the 4.5 hours run on.
        let plan = [(Driving, 2.0 * H), (Rest, 30.0), (Driving, 2.0 * H), (Rest, 15.0), (Driving, 1.0 * H)];
        let (summary, violations) = evaluate(&book(&plan), 0.0);
        assert_eq!(kinds(&violations), [ViolationKind::ContinuousDriving]);
        assert_eq!(summary.driving_since_break, 5.0 * H);
    }

    #[test]
    fn work_and_short_stops_are_not_breaks() {
        let plan = [(Driving, 3.0 * H), (Work, 60.0), (Rest, 10.0), (Driving, 2.0 * H)];
        let (_, violations) = evaluate(&book(&plan), 0.0);
        assert_eq!(kinds(&violations), [ViolationKind::ContinuousDriving]);
    }

    #[test]
    fn the_break_owed_after_a_first_fifteen() {
        // Resting now: it may become the whole 45.
        let (summary, _) = evaluate(&book(&[(Driving, 60.0), (Rest, 20.0)]), 0.0);
        assert_eq!(summary.break_needed, 45.0);
        assert_eq!(summary.break_taken, 20.0);
        let (summary, _) = evaluate(&book(&[(Driving, 60.0), (Rest, 20.0), (Driving, 10.0)]), 0.0);
        assert_eq!(summary.break_needed, 30.0);
        let (summary, _) = evaluate(&book(&[(Driving, 60.0), (Rest, 20.0), (Driving, 10.0), (Rest, 12.0)]), 0.0);
        assert_eq!(summary.break_needed, 30.0);
        assert_eq!(summary.break_taken, 12.0);
    }

    /// A day's driving: 4.5 hours at a time, 45 minutes' break between.
    fn driving(hours: f64) -> Vec<(Activity, f64)> {
        let mut plan = Vec::new();
        let mut left = hours * H;
        while left > 0.0 {
            let part = left.min(CONTINUOUS_DRIVING);
            plan.push((Driving, part));
            left -= part;
            if left > 0.0 {
                plan.push((Rest, BREAK_FULL));
            }
        }
        plan
    }

    /// A whole day: its driving, then rest until the next.
    fn full_day(hours: f64) -> Vec<(Activity, f64)> {
        let mut plan = driving(hours);
        let used: f64 = plan.iter().map(|(_, minutes)| minutes).sum();
        plan.push((Rest, MINUTES_PER_DAY - used));
        plan
    }

    #[test]
    fn nine_hours_a_day_ten_twice_a_week() {
        let mut plan = Vec::new();
        for _ in 0..2 {
            plan.extend(full_day(9.5)); // extended
        }
        // A third extended day breaks the rule once past 9 hours.
        plan.extend(driving(9.5));
        let (summary, violations) = evaluate(&book(&plan), 0.0);
        assert_eq!(summary.day_limit, DAILY_DRIVING);
        assert_eq!(kinds(&violations), [ViolationKind::DailyDriving]);
        assert_eq!(violations[0].over, 30.0);
        assert_eq!(summary.extended_days_used, 3);

        let (summary, violations) = evaluate(&book(&driving(8.5)), 0.0);
        assert_eq!(summary.day_limit, DAILY_DRIVING_EXTENDED);
        assert_eq!(summary.day_left, 600.0 - 510.0);
        assert!(violations.is_empty());
    }

    #[test]
    fn eleven_hours_daily_rest_within_twenty_four() {
        // 13 hours of a day, then 11 hours' rest: in time.
        let plan = [(Driving, 4.0 * H), (Rest, 60.0), (Driving, 4.0 * H), (Work, 4.0 * H), (Rest, 11.0 * H)];
        let (summary, violations) = evaluate(&book(&plan), 0.0);
        assert!(violations.is_empty(), "{violations:?}");
        assert_eq!(summary.day_driving, 0.0);

        // 17 hours on: a 9-hour rest no longer fits the 24.
        let plan = [(Driving, 4.0 * H), (Rest, 60.0), (Driving, 4.0 * H), (Work, 8.0 * H), (Rest, 9.0 * H)];
        let (_, violations) = evaluate(&book(&plan), 0.0);
        assert_eq!(kinds(&violations), [ViolationKind::DailyRestLate]);
    }

    #[test]
    fn three_reduced_rests_a_week() {
        let mut plan = Vec::new();
        for _ in 0..4 {
            plan.push((Driving, 4.0 * H));
            plan.push((Rest, 9.0 * H));
        }
        plan.push((Driving, 10.0));
        let (summary, violations) = evaluate(&book(&plan), 0.0);
        assert_eq!(kinds(&violations), [ViolationKind::ReducedRests]);
        assert_eq!(summary.reduced_rests_used, 4);

        // Split 3 + 9 is regular, not reduced.
        let mut plan = Vec::new();
        for _ in 0..4 {
            plan.extend([(Driving, 3.0 * H), (Rest, 3.0 * H), (Driving, 3.0 * H), (Rest, 9.0 * H)]);
        }
        plan.push((Driving, 10.0));
        let (summary, violations) = evaluate(&book(&plan), 0.0);
        assert!(violations.is_empty(), "{violations:?}");
        assert_eq!(summary.reduced_rests_used, 0);
    }

    #[test]
    fn fifty_six_a_week_ninety_in_two() {
        // Seven days of 8.5 hours: 56 is passed on the seventh.
        let mut plan = Vec::new();
        for _ in 0..7 {
            plan.extend(full_day(8.5));
        }
        let (summary, violations) = evaluate(&book(&plan), 0.0);
        assert_eq!(kinds(&violations), [ViolationKind::WeeklyDriving]);
        assert_eq!(violations[0].over, 7.0 * 8.5 * H - WEEKLY_DRIVING);
        assert_eq!(summary.week_driving, 0.0, "a new week has begun");
        assert_eq!(summary.fortnight_driving, 7.0 * 8.5 * H);

        // 55 one week, 40 the next: 90 passed in the second.
        let mut plan = Vec::new();
        for hours in [9.0, 9.0, 9.0, 9.0, 9.0, 10.0, 0.0, 9.0, 9.0, 9.0, 9.0, 4.0] {
            plan.extend(full_day(hours));
        }
        let (_, violations) = evaluate(&book(&plan), 0.0);
        assert_eq!(kinds(&violations), [ViolationKind::FortnightDriving]);
    }

    #[test]
    fn violations_recorded_once() {
        let mut book = book(&[(Driving, 5.0 * H)]);
        let (_, found) = evaluate(&book, 0.0);
        assert_eq!(book.record(&found).len(), 1);
        book.tick(Clock::Game, 5.0 * H + 1.0, Driving);
        let (_, found) = evaluate(&book, 0.0);
        assert!(book.record(&found).is_empty());
        assert_eq!(book.violations.len(), 1);
    }

    #[test]
    fn the_clock_skipping_is_rest_and_loading_a_save_is_nothing() {
        let mut book = book(&[(Driving, 60.0)]);
        // Sleep: the game's clock jumps 11 hours.
        assert!(book.tick(Clock::Game, 60.0 + 11.0 * H, Work));
        assert_eq!(book.activity(), Some(Rest));
        let (summary, _) = evaluate(&book, 0.0);
        assert_eq!(summary.day_driving, 0.0);
        // An older save: back in time, nothing counted.
        let now = book.now;
        assert!(!book.tick(Clock::Game, 30.0, Driving));
        assert_eq!(book.now, now);
        assert!(book.tick(Clock::Game, 31.0, Driving));
        assert_eq!(book.now, now + 1.0);
    }

    #[test]
    fn a_new_book_starts_at_the_clock() {
        let mut book = Book::default();
        assert!(!book.tick(Clock::Real, 1000.0, Driving));
        assert_eq!(book.now, 1000.0);
        book.tick(Clock::Real, 1000.5, Driving);
        assert_eq!(book.segments, [Segment { activity: Driving, start: 1000.0, end: 1000.5 }]);
        // Euro-Hub closed an hour: rest.
        book.tick(Clock::Real, 1060.5, Driving);
        assert_eq!(book.activity(), Some(Rest));
    }

    #[test]
    fn weeks_start_on_monday() {
        // 2026-11-16 (a Monday), 00:00 UTC.
        let monday = 20_773.0 * MINUTES_PER_DAY;
        let origin = week_origin(Clock::Real, 0.0);
        assert_eq!(week_of(monday, origin), week_of(monday + MINUTES_PER_WEEK - 1.0, origin));
        assert_eq!(week_of(monday - 1.0, origin) + 1, week_of(monday, origin));
        // An hour ahead of UTC: the week starts an hour earlier in UTC.
        let origin = week_origin(Clock::Real, 60.0);
        assert_eq!(week_of(monday - 30.0, origin), week_of(monday, origin));
    }

    #[test]
    fn warns_fifteen_and_five_minutes_ahead_and_when_reached() {
        let mut warner = Warner::default();
        let at = |left: f64| Summary { activity: Some(Driving), next_left: left, ..Summary::default() };
        assert_eq!(warner.update(&at(20.0)), None);
        assert_eq!(warner.update(&at(15.0)), Some(Warning::Soon(Limit::Break, 15)));
        assert_eq!(warner.update(&at(10.0)), None);
        assert_eq!(warner.update(&at(4.9)), Some(Warning::Soon(Limit::Break, 5)));
        assert_eq!(warner.update(&at(0.0)), Some(Warning::Reached(Limit::Break)));
        assert_eq!(warner.update(&at(-1.0)), None);
    }

    #[test]
    fn the_detector_waits_before_changing() {
        let start = std::time::Instant::now();
        let mut detector = Detector::default();
        assert_eq!(detector.update(Driving, start), Driving);
        assert_eq!(detector.update(Rest, start + std::time::Duration::from_secs(1)), Driving);
        assert_eq!(detector.update(Rest, start + std::time::Duration::from_secs(5)), Rest);
        assert_eq!(Detector::seen(10.0, true), Driving);
        assert_eq!(Detector::seen(0.0, true), Work);
        assert_eq!(Detector::seen(0.1, false), Rest);
    }

    #[test]
    fn the_card_number_is_each_profiles_own() {
        assert_eq!(card_number("4465204C6572656E2055696C"), card_number("4465204C6572656E2055696C"));
        assert_ne!(card_number("4465204C6572656E2055696C"), card_number("54657374"));
        assert_eq!(card_number("54657374").len(), 16);
    }

    #[test]
    fn the_profile_from_its_files() {
        let profile = westplay_mapdata::sii::parse(
            "SiiNunit
{
user_profile : _nameless.1 {
 profile_name: \"De Leren Uil\"
 company_name: \"Kronos vervoer\"
 cached_distance: 4171
}
}
",
        );
        let save = westplay_mapdata::sii::parse(
            "SiiNunit
{
bank : _nameless.2 {
 money_account: 138121
}
delivery_log : _nameless.3 {
 cached_jobs_count: 12
}
economy : _nameless.4 {
 experience_points: 219
}
}
",
        );
        let read = Profile::read("4465204C6572656E2055696C", &profile, &save).unwrap();
        assert_eq!(read.name, "De Leren Uil");
        assert_eq!(read.company, "Kronos vervoer");
        assert_eq!((read.distance, read.deliveries, read.experience, read.money), (4171, 12, 219, 138121));
        assert!(Profile::read("x", &[], &save).is_none());
    }

    #[test]
    fn the_book_survives_a_restart() {
        let dir = std::env::temp_dir().join(format!("dashly-tacho-{}", std::process::id()));
        let book = book(&[(Driving, 90.0), (Rest, 20.0)]);
        book.save(&dir, Clock::Game).unwrap();
        assert_eq!(Book::load(&dir, Clock::Game), book);
        assert_eq!(Book::load(&dir, Clock::Real), Book::default());
        let _ = std::fs::remove_dir_all(dir);
    }
}
