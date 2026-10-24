//! S-3511A retained registers. Transport and CPU clocks are deliberately separate.
use crate::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Rtc {
    // Binary fields except seconds, which may temporarily contain invalid BCD.
    date: [u8; 6], // year, month, day, weekday, hour (0..23), minute
    second: u8,
    status: u8,
    alarm: [u8; 2],
    fraction: u16,
    anchor: Option<u64>,
}
impl Default for Rtc {
    fn default() -> Self {
        Self {
            date: [0, 1, 1, 0, 0, 0],
            second: 0,
            status: 0x82,
            alarm: [0x80, 0],
            fraction: 0,
            anchor: None,
        }
    }
}
fn bcd(n: u8) -> u8 {
    (n / 10) * 16 + n % 10
}
fn decode(n: u8) -> Option<u8> {
    (n & 15 < 10 && n >> 4 < 10).then_some((n >> 4) * 10 + (n & 15))
}
fn field(n: u8, min: u8, max: u8, fallback: u8) -> u8 {
    decode(n)
        .filter(|v| (min..=max).contains(v))
        .unwrap_or(fallback)
}
fn days(year: u8, month: u8) -> u8 {
    match month {
        2 => {
            if year.is_multiple_of(4) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}
impl Rtc {
    pub fn reset(&mut self) {
        let anchor = self.anchor;
        *self = Self {
            status: 0,
            alarm: [0; 2],
            anchor,
            ..Self::default()
        };
    }
    pub fn snapshot(&self, command: u8) -> [u8; 7] {
        let mut data = [0xff; 7];
        match command & 14 {
            2 => data[0] = self.status,
            4 | 6 => {
                let [year, month, day, weekday, hour, minute] = self.date;
                let h = bcd(if self.status & 0x40 != 0 {
                    hour
                } else {
                    hour % 12
                }) | if hour >= 12 { 0x80 } else { 0 };
                let full = [
                    bcd(year),
                    bcd(month),
                    bcd(day),
                    weekday,
                    h,
                    bcd(minute),
                    self.second,
                ];
                if command & 14 == 4 {
                    data = full;
                } else {
                    data[..3].copy_from_slice(&full[4..]);
                }
            }
            _ => {} // Reset has no payload; INT registers are write-only.
        }
        data
    }
    pub fn write(&mut self, command: u8, data: &[u8; 7]) {
        match command & 15 {
            2 => self.status = (self.status & 0x80) | (data[0] & 0x6a),
            4 | 6 => {
                let start = if command & 15 == 4 { 0 } else { 4 };
                if start == 0 {
                    self.date[0] = field(data[0], 0, 99, 0);
                    self.date[1] = field(data[1] & 0x1f, 1, 12, 1);
                    self.date[2] = field(data[2] & 0x3f, 1, 31, 1);
                    self.date[3] = if data[3] & 7 == 7 { 0 } else { data[3] & 7 };
                    if self.date[2] > days(self.date[0], self.date[1]) {
                        self.date[2] = 1;
                        self.next_month();
                    }
                }
                let h = field(
                    data[4 - start] & 0x3f,
                    0,
                    if self.status & 0x40 != 0 { 23 } else { 11 },
                    0,
                );
                self.date[4] = if self.status & 0x40 == 0 && data[4 - start] & 0x80 != 0 {
                    h + 12
                } else {
                    h
                };
                self.date[5] = field(data[5 - start] & 0x7f, 0, 59, 0);
                self.second = data[6 - start] & 0x7f;
                self.fraction = 0;
            }
            8 | 10 => self.alarm.copy_from_slice(&data[..2]),
            _ => {}
        }
    }
    fn next_month(&mut self) {
        self.date[1] += 1;
        if self.date[1] == 13 {
            self.date[1] = 1;
            self.date[0] = (self.date[0] + 1) % 100;
        }
    }
    pub fn sync(&mut self, now: u64) {
        if let Some(previous) = self.anchor {
            if now <= previous {
                return;
            }
            self.advance(now - previous);
        }
        self.anchor = Some(now);
    }
    pub fn advance(&mut self, milliseconds: u64) {
        let fractional = u64::from(self.fraction) + milliseconds % 1000;
        self.fraction = (fractional % 1000) as u16;
        let seconds = milliseconds / 1000 + fractional / 1000;
        if seconds == 0 {
            return;
        }
        // Invalid seconds correct to zero on the first carry, with minute carry.
        let initial = decode(self.second).filter(|s| *s < 60);
        let total = initial.map_or(seconds - 1 + 60, |s| seconds + u64::from(s))
            + u64::from(self.date[5]) * 60
            + u64::from(self.date[4]) * 3600;
        self.second = bcd((total % 60) as u8);
        self.date[5] = (total / 60 % 60) as u8;
        self.date[4] = (total / 3600 % 24) as u8;
        let elapsed_days = total / 86400;
        self.date[3] = ((u64::from(self.date[3]) + elapsed_days) % 7) as u8;
        // Repeat the documented 2000–2099 cycle. At most 36,524 day steps,
        // even for u64::MAX milliseconds; never execute game frames to catch up.
        for _ in 0..elapsed_days % 36525 {
            self.date[2] += 1;
            if self.date[2] > days(self.date[0], self.date[1]) {
                self.date[2] = 1;
                self.next_month();
            }
        }
    }
    pub fn export(&self) -> Vec<u8> {
        let mut data = b"RTC\x01".to_vec();
        data.extend(self.date);
        data.extend([self.second, self.status, self.alarm[0], self.alarm[1]]);
        data.extend(self.fraction.to_le_bytes());
        data.push(u8::from(self.anchor.is_some()));
        data.extend(self.anchor.unwrap_or(0).to_le_bytes());
        data
    }
    pub fn import(data: &[u8]) -> Result<Self, Error> {
        let invalid = || Error::new("Invalid or unsupported RTC record.");
        if data.len() != 25 || &data[..4] != b"RTC\x01" {
            return Err(invalid());
        }
        let date: [u8; 6] = data[4..10].try_into().unwrap();
        let fraction = u16::from_le_bytes(data[14..16].try_into().unwrap());
        let anchor = u64::from_le_bytes(data[17..25].try_into().unwrap());
        if date[0] > 99
            || !(1..=12).contains(&date[1])
            || date[2] == 0
            || date[2] > days(date[0], date[1])
            || date[3] > 6
            || date[4] > 23
            || date[5] > 59
            || data[10] > 0x7f
            || data[11] & !0xea != 0
            || fraction >= 1000
            || data[16] > 1
            || (data[16] == 0 && anchor != 0)
        {
            return Err(invalid());
        }
        Ok(Self {
            date,
            second: data[10],
            status: data[11],
            alarm: [data[12], data[13]],
            fraction,
            anchor: (data[16] == 1).then_some(anchor),
        })
    }
}
