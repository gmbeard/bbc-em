const CYCLES_PER_SECOND: usize = 2_000_000;
const CYCLES_PER_MS: usize = CYCLES_PER_SECOND / 1000;
const TIMER_INTERVAL_IN_MS: usize = 20;
const CYCLES_PER_INTERVAL: usize = CYCLES_PER_MS * TIMER_INTERVAL_IN_MS;

pub struct Timer {
    elapsed_cycles: usize
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            elapsed_cycles: 0
        }
    }

    pub fn step(&mut self, cycles: usize) -> bool {
        self.elapsed_cycles += cycles;
        if self.elapsed_cycles >= CYCLES_PER_INTERVAL {
            self.elapsed_cycles %= CYCLES_PER_INTERVAL;
            return true;
        }

        false
    }
}

