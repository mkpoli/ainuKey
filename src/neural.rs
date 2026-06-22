//! Pure-Rust neural next-word model — a streaming single-layer LSTM, no ONNX
//! runtime or other native dependency.
//!
//! Loads the `lstm.bin` produced by `tools/neural/export_weights.py`: the four
//! big matrices (embedding, the two LSTM weight matrices, the output projection)
//! are per-tensor symmetric **int8** with an f32 scale, dequantized to f32 once
//! at load; biases are f32. Inference keeps a small `(h, c)` [`State`] across the
//! committed words of a composition, steps it one word at a time, and ranks the
//! vocab words matching the typed prefix by the next-word logits — the same
//! candidate shape the n-gram engine produces, but with full-sentence context.
//!
//! Binary layout (little-endian, version 1), matching `export_weights.py`:
//! ```text
//! magic "AKLS" | u32 version
//! u32 V | u32 E | u32 H | u8 layers(=1) | u8 quant(0=f32,1=int8)
//! vocab: V × { u8 len, utf8 bytes }
//! big matrices  emb[V,E] Wih[4H,E] Whh[4H,H] projW[V,H] (row-major):
//!    int8 → f32 scale then rows*cols × i8   |  f32 → rows*cols × f32
//! bias vectors  bih[4H] bhh[4H] projB[V] : rows × f32
//! ```
//!
//! Pure `std`, so it unit-tests on any host (a synthetic tiny model checks the
//! math; the real model is parity-checked against the Python exporter offline).
#![allow(dead_code)]

use std::collections::HashMap;

const MAGIC: &[u8; 4] = b"AKLS";
const VERSION: u32 = 1;

/// A parsed LSTM model. All weights are dequantized f32.
pub struct Model {
    v: usize,
    e: usize,
    h: usize,
    itos: Vec<String>,
    stoi: HashMap<String, u32>,
    /// `prefix → vocab ids (real words, id ≥ 4) starting with it`, frequency-free.
    prefix: HashMap<String, Vec<u32>>,
    emb: Vec<f32>,   // [V*E]
    wih: Vec<f32>,   // [4H*E]
    whh: Vec<f32>,   // [4H*H]
    bih: Vec<f32>,   // [4H]
    bhh: Vec<f32>,   // [4H]
    projw: Vec<f32>, // [V*H]
    projb: Vec<f32>, // [V]
}

/// Streaming LSTM state — carry it across the committed words of a composition.
#[derive(Clone)]
pub struct State {
    h: Vec<f32>,
    c: Vec<f32>,
}

impl State {
    /// The hidden vector (exposed for parity testing against the exporter).
    pub fn hidden(&self) -> &[f32] {
        &self.h
    }
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.b.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn string(&mut self) -> Option<String> {
        let n = self.u8()? as usize;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
    /// `n` f32s, either raw or dequantized from int8 with a leading f32 scale.
    fn matrix(&mut self, n: usize, quant: bool) -> Option<Vec<f32>> {
        if quant {
            let scale = self.f32()?;
            let raw = self.take(n)?;
            Some(raw.iter().map(|&q| (q as i8) as f32 * scale).collect())
        } else {
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                out.push(self.f32()?);
            }
            Some(out)
        }
    }
}

#[inline]
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Dot product of `a[off..off+n]` with `b[0..n]`.
#[inline]
fn dot(a: &[f32], off: usize, b: &[f32]) -> f32 {
    let row = &a[off..off + b.len()];
    row.iter().zip(b).map(|(x, y)| x * y).sum()
}

impl Model {
    /// Parse a model from raw bytes; `None` if the bytes are not a valid table
    /// (wrong magic/version or truncated) so a bad file disables the engine
    /// rather than panicking.
    pub fn load(bytes: &[u8]) -> Option<Model> {
        let mut r = Reader::new(bytes);
        if r.take(4)? != MAGIC || r.u32()? != VERSION {
            return None;
        }
        let v = r.u32()? as usize;
        let e = r.u32()? as usize;
        let h = r.u32()? as usize;
        let _layers = r.u8()?;
        let quant = r.u8()? != 0;
        if v == 0 || e == 0 || h == 0 {
            return None;
        }
        let mut itos = Vec::with_capacity(v);
        for _ in 0..v {
            itos.push(r.string()?);
        }
        let emb = r.matrix(v * e, quant)?;
        let wih = r.matrix(4 * h * e, quant)?;
        let whh = r.matrix(4 * h * h, quant)?;
        let projw = r.matrix(v * h, quant)?;
        let bih = r.matrix(4 * h, false)?;
        let bhh = r.matrix(4 * h, false)?;
        let projb = r.matrix(v, false)?;

        let mut stoi = HashMap::with_capacity(v);
        let mut prefix: HashMap<String, Vec<u32>> = HashMap::new();
        for (i, w) in itos.iter().enumerate() {
            stoi.insert(w.clone(), i as u32);
            if i >= 4 {
                // skip <pad><unk><bos><eos>
                let chars: Vec<char> = w.chars().collect();
                let mut p = String::new();
                for ch in chars {
                    p.push(ch);
                    prefix.entry(p.clone()).or_default().push(i as u32);
                }
            }
        }
        Some(Model {
            v, e, h, itos, stoi, prefix, emb, wih, whh, bih, bhh, projw, projb,
        })
    }

    pub fn vocab_size(&self) -> usize {
        self.v
    }

    /// The `<bos>` token id — feed it to start a fresh sentence context.
    pub fn bos(&self) -> u32 {
        self.stoi.get("<bos>").copied().unwrap_or(2)
    }

    /// A fresh zero state (start of a composition).
    pub fn new_state(&self) -> State {
        State {
            h: vec![0.0; self.h],
            c: vec![0.0; self.h],
        }
    }

    /// Advance the state by one already-committed word (or its `<unk>` = id 1).
    pub fn step_word(&self, st: &mut State, word: &str) {
        self.step_id(st, self.stoi.get(word).copied().unwrap_or(1));
    }

    /// Advance the state by one token id.
    pub fn step_id(&self, st: &mut State, id: u32) {
        let id = (id as usize).min(self.v - 1);
        let e = &self.emb[id * self.e..id * self.e + self.e];
        let hh = self.h;
        // Gate preactivations [4H] in PyTorch order: input, forget, cell, output.
        let g: Vec<f32> = (0..4 * hh)
            .map(|k| {
                self.bih[k] + self.bhh[k]
                    + dot(&self.wih, k * self.e, e)
                    + dot(&self.whh, k * hh, &st.h)
            })
            .collect();
        for j in 0..hh {
            let i = sigmoid(g[j]);
            let f = sigmoid(g[hh + j]);
            let gg = g[2 * hh + j].tanh();
            let o = sigmoid(g[3 * hh + j]);
            st.c[j] = f * st.c[j] + i * gg;
            st.h[j] = o * st.c[j].tanh();
        }
    }

    /// Full next-word logits given the current state (used by tests / ranking).
    pub fn logits(&self, st: &State) -> Vec<f32> {
        (0..self.v)
            .map(|t| self.projb[t] + dot(&self.projw, t * self.h, &st.h))
            .collect()
    }

    /// The `k` vocab words starting with `prefix`, ranked by next-word logit given
    /// the context `st` (best first). Empty prefix or no matches → empty. Only the
    /// matching rows of the projection are scored, so this is cheap per keystroke.
    pub fn complete(&self, st: &State, prefix: &str, k: usize) -> Vec<&str> {
        let Some(cands) = self.prefix.get(prefix) else {
            return Vec::new();
        };
        let mut scored: Vec<(f32, u32)> = cands
            .iter()
            .map(|&id| {
                let s = self.projb[id as usize]
                    + dot(&self.projw, id as usize * self.h, &st.h);
                (s, id)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored
            .into_iter()
            .take(k)
            .map(|(_, id)| self.itos[id as usize].as_str())
            .collect()
    }
}

/// The embedded model, generated into `OUT_DIR` by `build.rs`: the real
/// `data/neural/lstm.bin` when present, otherwise an empty header — so a public
/// build (without the corpus-derived model) compiles with the neural engine
/// simply disabled.
const TABLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/lstm.bin"));

/// The process-wide model, parsed once. `None` when the embedded table is the
/// empty placeholder (no model shipped) or somehow corrupt.
pub fn global() -> Option<&'static Model> {
    use std::sync::OnceLock;
    static G: OnceLock<Option<Model>> = OnceLock::new();
    G.get_or_init(|| Model::load(TABLE)).as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a tiny valid int8-free model by hand: V=5, E=1, H=1, identity-ish
    /// weights, so the LSTM math can be checked against a hand computation.
    fn synth() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(MAGIC);
        b.extend(VERSION.to_le_bytes());
        let (v, e, h) = (5u32, 1u32, 1u32);
        b.extend(v.to_le_bytes());
        b.extend(e.to_le_bytes());
        b.extend(h.to_le_bytes());
        b.push(1); // layers
        b.push(0); // quant = f32
        for w in ["<pad>", "<unk>", "<bos>", "<eos>", "ku"] {
            b.push(w.len() as u8);
            b.extend(w.as_bytes());
        }
        let put = |b: &mut Vec<u8>, xs: &[f32]| {
            for x in xs {
                b.extend(x.to_le_bytes());
            }
        };
        // emb [V*E=5]: token id 4 ("ku") has embedding 1.0, others 0.
        put(&mut b, &[0.0, 0.0, 0.0, 0.0, 1.0]);
        // Wih [4H*E = 4]: gates i,f,g,o weights on the single input.
        put(&mut b, &[0.0, 0.0, 2.0, 0.0]); // only the cell gate reacts to input
        // Whh [4H*H = 4]
        put(&mut b, &[0.0, 0.0, 0.0, 0.0]);
        // projW [V*H = 5]: logit_t = projw[t]*h
        put(&mut b, &[0.0, 0.0, 0.0, 0.0, 3.0]);
        // bih [4H], bhh [4H]: bias the input gate open (i≈1) so c≈g.
        put(&mut b, &[10.0, -10.0, 0.0, 10.0]); // i open, f closed, o open
        put(&mut b, &[0.0, 0.0, 0.0, 0.0]);
        // projB [V]
        put(&mut b, &[0.0, 0.0, 0.0, 0.0, 0.5]);
        b
    }

    #[test]
    fn parses_and_steps() {
        let m = Model::load(&synth()).expect("valid");
        assert_eq!(m.vocab_size(), 5);
        let mut st = m.new_state();
        m.step_word(&mut st, "ku"); // feeds embedding 1.0
        // cell gate g = tanh(Wih_g * 1) = tanh(2) ≈ 0.964; i≈1, f≈0 → c≈0.964;
        // o≈1 → h ≈ tanh(0.964) ≈ 0.7459.
        let h = &st.h[0];
        assert!((h - 0.7459).abs() < 1e-3, "h={h}");
        // logit("ku") = projw[4]*h + projb[4] = 3*h + 0.5 ≈ 2.738.
        let lg = m.logits(&st);
        assert!((lg[4] - (3.0 * 0.7459 + 0.5)).abs() < 1e-2, "{:?}", lg);
        // completion of "k" → "ku" (the only matching word).
        assert_eq!(m.complete(&st, "k", 5), vec!["ku"]);
        assert!(m.complete(&st, "z", 5).is_empty());
    }

    #[test]
    fn rejects_garbage() {
        assert!(Model::load(b"nope").is_none());
        assert!(Model::load(b"AKLS\x01").is_none());
        assert!(Model::load(&[]).is_none());
    }
}
