// SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
//
// SPDX-License-Identifier: Apache-2.0

/// A byte-oriented ring buffer with a monotonically increasing sequence counter.
///
/// The buffer stores at most `cap` bytes. New data overwrites old data when full.
/// Each written byte increases the global `seq` counter by 1.
///
/// Sequence numbers allow readers (e.g. WebSocket clients) to keep a cursor and
/// request "all data since cursor".
pub struct ByteRing {
    buf: Vec<u8>,
    cap: usize,
    head: usize,
    seq: u64,
    len: usize,
}

impl ByteRing {
    /// Create a new empty ring buffer with the given capacity in bytes.
    pub fn new(cap: usize) -> Self {
        Self {
            buf: vec![0; cap],
            cap,
            head: 0,
            seq: 0,
            len: 0,
        }
    }

    /// Return the current global sequence number (i.e. "end cursor").
    pub fn current_seq(&self) -> u64 {
        self.seq
    }

    /// Return the sequence number of the oldest byte still stored.
    pub fn oldest_seq(&self) -> u64 {
        self.seq - self.len as u64
    }

    /// Remove all currently stored data from the buffer.
    ///
    /// Notes:
    /// - This resets `len` and `head`, but keeps the allocated memory.
    /// - By default this does NOT reset `seq` (monotonicity is preserved).
    ///   This is important for WebSocket cursors: after a clear, clients will
    ///   simply observe an empty range until new bytes arrive.
    pub fn clear(&mut self) {
        self.len = 0;
        self.head = 0;
        // keep self.seq monotonic on purpose
    }

    /// Push new bytes into the ring buffer.
    pub fn push(&mut self, data: &[u8]) {
        for &b in data {
            self.buf[self.head] = b;
            self.head = (self.head + 1) % self.cap;
            self.seq += 1;
            if self.len < self.cap {
                self.len += 1;
            }
        }
    }

    /// Return a snapshot of the entire currently stored buffer content,
    /// ordered from oldest to newest.
    pub fn snapshot_all(&self) -> Vec<u8> {
        use std::cmp::min;

        let mut out = Vec::with_capacity(self.len);
        let start = (self.head + self.cap - self.len) % self.cap;
        let first = min(self.len, self.cap - start);
        out.extend_from_slice(&self.buf[start..start + first]);
        if first < self.len {
            out.extend_from_slice(&self.buf[0..(self.len - first)]);
        }
        out
    }

    /// Read data starting from the given sequence cursor.
    ///
    /// Returns (bytes, new_cursor, lagged).
    pub fn read_from(&self, cursor: u64, max_chunk: usize) -> (Vec<u8>, u64, bool) {
        use std::cmp::min;

        let oldest = self.oldest_seq();
        let newest = self.current_seq();

        let mut cur = cursor;
        let lagged = cur < oldest;
        if lagged {
            cur = oldest;
        }

        if cur >= newest {
            return (Vec::new(), newest, lagged);
        }

        let avail = (newest - cur) as usize;
        let take = min(avail, max_chunk);

        let start_index = (self.head + self.cap - self.len) % self.cap;
        let offset = (cur - oldest) as usize;
        let idx = (start_index + offset) % self.cap;

        let mut out = Vec::with_capacity(take);
        let first = min(take, self.cap - idx);
        out.extend_from_slice(&self.buf[idx..idx + first]);
        if first < take {
            out.extend_from_slice(&self.buf[0..(take - first)]);
        }

        (out, cur + take as u64, lagged)
    }
}

#[cfg(test)]
mod tests {
    use super::ByteRing;

    #[test]
    fn new_is_empty_and_seq_is_zero() {
        // Requirement:
        // - Newly created buffer has seq = 0
        // - len = 0
        // - oldest_seq = 0
        // - snapshot_all() is empty
        // - read_from(0) returns empty without lag

        let r = ByteRing::new(8);

        assert_eq!(r.current_seq(), 0);
        assert_eq!(r.oldest_seq(), 0);
        assert_eq!(r.snapshot_all(), Vec::<u8>::new());

        let (b, newc, lagged) = r.read_from(0, 1024);
        assert!(b.is_empty());
        assert_eq!(newc, 0);
        assert!(!lagged);
    }

    #[test]
    fn push_without_wrap_snapshot_all_ordered() {
        // Requirement:
        // - Pushing less than capacity stores bytes in order
        // - seq increases per byte
        // - snapshot_all returns oldest → newest
        // - read_from(0) returns all data

        let mut r = ByteRing::new(8);
        r.push(b"abc");

        assert_eq!(r.current_seq(), 3);
        assert_eq!(r.oldest_seq(), 0);
        assert_eq!(r.snapshot_all(), b"abc".to_vec());

        let (b, newc, lagged) = r.read_from(0, 1024);
        assert_eq!(b, b"abc".to_vec());
        assert_eq!(newc, 3);
        assert!(!lagged);
    }

    #[test]
    fn overwrite_when_full_snapshot_keeps_last_cap_bytes() {
        // Requirement:
        // - When capacity exceeded, oldest bytes are overwritten
        // - Buffer stores only last `cap` bytes
        // - oldest_seq reflects dropped bytes
        // - seq remains monotonic

        let mut r = ByteRing::new(4);
        r.push(b"abcdef"); // cap=4 → keeps "cdef"

        assert_eq!(r.current_seq(), 6);
        assert_eq!(r.oldest_seq(), 2);
        assert_eq!(r.snapshot_all(), b"cdef".to_vec());

        let (b, newc, lagged) = r.read_from(r.oldest_seq(), 1024);
        assert_eq!(b, b"cdef".to_vec());
        assert_eq!(newc, 6);
        assert!(!lagged);
    }

    #[test]
    fn read_from_in_chunks_advances_cursor() {
        // Requirement:
        // - read_from returns at most max_chunk bytes
        // - returned cursor advances correctly
        // - subsequent reads continue from new cursor
        // - reading at newest returns empty

        let mut r = ByteRing::new(16);
        r.push(b"hello");

        let (b1, c1, _) = r.read_from(0, 2);
        assert_eq!(b1, b"he".to_vec());
        assert_eq!(c1, 2);

        let (b2, c2, _) = r.read_from(c1, 2);
        assert_eq!(b2, b"ll".to_vec());
        assert_eq!(c2, 4);

        let (b3, c3, _) = r.read_from(c2, 10);
        assert_eq!(b3, b"o".to_vec());
        assert_eq!(c3, 5);

        let (b4, c4, lagged) = r.read_from(c3, 10);
        assert!(b4.is_empty());
        assert_eq!(c4, 5);
        assert!(!lagged);
    }

    #[test]
    fn read_from_past_newest_returns_empty_cursor_is_newest() {
        // Requirement:
        // - If cursor >= current_seq, return empty
        // - Cursor is normalized to newest
        // - No lag if cursor is ahead

        let mut r = ByteRing::new(8);
        r.push(b"xyz");

        let (b, newc, lagged) = r.read_from(999, 10);
        assert!(b.is_empty());
        assert_eq!(newc, r.current_seq());
        assert!(!lagged);
    }

    #[test]
    fn lagged_cursor_is_clamped_to_oldest_and_flagged() {
        // Requirement:
        // - If cursor < oldest_seq, lagged=true
        // - Cursor clamped to oldest_seq
        // - Returned data starts from oldest

        let mut r = ByteRing::new(4);
        r.push(b"abcdef"); // keeps "cdef"

        let (b, newc, lagged) = r.read_from(0, 10);
        assert!(lagged);
        assert_eq!(b, b"cdef".to_vec());
        assert_eq!(newc, 6);
    }

    #[test]
    fn wraparound_read_returns_correct_order() {
        // Requirement:
        // - Buffer wraparound does not break ordering
        // - snapshot_all remains oldest → newest
        // - read_from works correctly across wrap boundary

        let mut r = ByteRing::new(5);
        r.push(b"ABCD");
        r.push(b"EFGH"); // last 5 → "DEFGH"

        assert_eq!(r.snapshot_all(), b"DEFGH".to_vec());

        let (b1, c1, _) = r.read_from(r.oldest_seq(), 2);
        assert_eq!(b1, b"DE".to_vec());

        let (b2, c2, _) = r.read_from(c1, 10);
        assert_eq!(b2, b"FGH".to_vec());
        assert_eq!(c2, 8);
    }

    #[test]
    fn clear_resets_len_and_head_but_keeps_seq_monotonic() {
        // Requirement:
        // - clear() removes stored data
        // - seq remains monotonic (not reset)
        // - oldest_seq == current_seq after clear
        // - read_from with old cursor reports lag

        let mut r = ByteRing::new(4);
        r.push(b"abc");
        r.clear();

        assert_eq!(r.current_seq(), 3);
        assert_eq!(r.oldest_seq(), 3);
        assert_eq!(r.snapshot_all(), Vec::<u8>::new());

        let (b, newc, lagged) = r.read_from(0, 10);
        assert!(b.is_empty());
        assert_eq!(newc, 3);
        assert!(lagged);
    }

    #[test]
    fn clear_then_push_continues_seq_and_reads_only_new_data() {
        // Requirement:
        // - After clear, new data continues seq
        // - Old data is not visible
        // - Cursor at pre-clear seq reads only new bytes

        let mut r = ByteRing::new(4);
        r.push(b"abcd"); // seq=4
        r.clear();
        r.push(b"XY"); // seq=6

        assert_eq!(r.snapshot_all(), b"XY".to_vec());

        let (b, newc, lagged) = r.read_from(4, 10);
        assert_eq!(b, b"XY".to_vec());
        assert_eq!(newc, 6);
        assert!(!lagged);

        let (b2, newc2, lagged2) = r.read_from(0, 10);
        assert_eq!(b2, b"XY".to_vec());
        assert_eq!(newc2, 6);
        assert!(lagged2);
    }

    #[test]
    fn max_chunk_zero_returns_empty_but_preserves_cursor_rules() {
        // Requirement:
        // - max_chunk = 0 returns empty
        // - Cursor remains unchanged (or clamped if lagged)
        // - Lag detection still works

        let mut r = ByteRing::new(8);
        r.push(b"hi");

        let (b, newc, lagged) = r.read_from(0, 0);
        assert!(b.is_empty());
        assert_eq!(newc, 0);
        assert!(!lagged);

        let mut r2 = ByteRing::new(2);
        r2.push(b"abcd"); // oldest=2

        let (b2, newc2, lagged2) = r2.read_from(0, 0);
        assert!(b2.is_empty());
        assert_eq!(newc2, 2);
        assert!(lagged2);
    }
}
