#[derive(Clone, Debug)]
pub struct Animation {
    start_time: f32,
    end_time: f32,
    start: f32,
    end: f32,
    m: f32,
    c: f32,
}

impl Default for Animation {
    fn default() -> Self {
        Self::empty()
    }
}

impl Animation {
    pub fn new(start_time: f32, duration: f32, start: f32, end: f32) -> Self {
        let m = (end - start) / duration;
        let c = start - m * start_time;
        Animation {
            start_time,
            end_time: start_time + duration,
            start,
            end,
            m,
            c,
        }
    }

    pub fn empty() -> Self {
        Animation {
            start_time: 1.0,
            end_time: 1.0,
            start: 0.0,
            end: 0.0,
            m: 0.0,
            c: 0.0,
        }
    }

    pub fn current_delta(&mut self, cur_time: f32) -> f32 {
        if cur_time < self.start_time {
            return self.start;
        }

        if cur_time > self.end_time {
            self.start = self.end;
            self.m = 0.0;
            self.c = self.end;
            return self.end;
        }

        self.m * cur_time + self.c
    }
}
