use log::debug;
use std::sync::{Arc, Mutex};

use clokwerk::{Interval, Job, TimeUnits};

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum ScheduledActivity {
    TakePills,
    WaterPlants,
    TestActivity,
}

pub(crate) struct Scheduler {
    clokwerk: clokwerk::Scheduler,
    buf: Arc<Mutex<Vec<ScheduledActivity>>>,
}

impl Scheduler {
    pub(crate) fn new() -> Scheduler {
        let mut clokwerk = clokwerk::Scheduler::new();

        // This looks completely insane:
        // 1. There aren't any threads here so why is the run method requiring the
        //    callback to have `Send` - this is forcing me to use the Arc<Mutex<..>>
        // 2. Is this really the simplest way to get the data in and out of the buffer?
        //    Looks absolutely bonkers.
        // TODO Learn to rust better / find a better library / DIY it.
        let buf: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(Vec::new()));

        let buf0 = buf.clone();
        clokwerk.every(1.day()).at("6:00am").run(move || {
            // TODO Improve logging everywhere, this should be at debug! with debug! in
            // tick and then info! in main.
            debug!("Scheduler triggering TakePills");
            buf0.lock().unwrap().push(ScheduledActivity::TakePills);
        });

        let buf1 = buf.clone();
        clokwerk
            .every(Interval::Saturday)
            .at("8:00am")
            .run(move || {
                debug!("Scheduler triggering WaterPlants");
                buf1.lock().unwrap().push(ScheduledActivity::WaterPlants);
            });

        let buf2 = buf.clone();
        clokwerk
            .every(Interval::Minutes(5))
            .run(move || {
                debug!("Scheduler triggering TestActivity");
                buf2.lock().unwrap().push(ScheduledActivity::TestActivity);
            });

        Scheduler { clokwerk, buf: buf }
    }

    pub(crate) fn tick(self: &mut Self) -> Vec<ScheduledActivity> {
        self.clokwerk.run_pending();
        let mut buf = self.buf.lock().unwrap();
        let result = buf.clone();
        buf.clear();
        result
    }
}
