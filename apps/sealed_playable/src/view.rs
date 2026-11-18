//! Bounded read-only projection of *validated public* C0.3 checkpoint bytes.
//! No rules, mutation, path search, or simulation state is implemented here.
use sealed_sim::Simulation;

pub const MAX_CHECKPOINT: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorView {
    pub id: u32,
    pub enabled: u8,
    pub mode: u8,
    pub next_wake: u64,
    pub needs: [u32; 5],
    pub home: u64,
    pub task: u64,
    pub locus: u8,
    pub site: u64,
    pub x: u32,
    pub y: u32,
    pub edge: u64,
    pub traversed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StockView {
    pub id: u64,
    pub definition: u64,
    pub class: u8,
    pub quantity: u64,
    pub owner_kind: u8,
    pub owner: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskView {
    pub id: u64,
    pub actor: u32,
    pub goal: u8,
    pub stage: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub actors: Vec<ActorView>,
    pub sites: Vec<u64>,
    pub edges: Vec<(u64, u64, u64, u64)>,
    pub grid: GridView,
    pub stocks: Vec<StockView>,
    pub tasks: Vec<TaskView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridView {
    pub site: u64,
    pub width: u32,
    pub height: u32,
    pub passable: Vec<u8>,
}

impl View {
    /// Public ingress is checked by the authoritative kernel before projection.
    pub fn validated(bytes: &[u8]) -> Result<(Simulation, Self), String> {
        if bytes.len() > MAX_CHECKPOINT || bytes.get(..12) != Some(b"SH:C03:STATE") {
            return Err("checkpoint.layout".into());
        }
        let sim = Simulation::restore_domain_checkpoint(bytes)
            .map_err(|e| format!("checkpoint.invalid: {e:?}"))?;
        let view = Self::from_sim(&sim)?;
        Ok((sim, view))
    }

    pub fn from_sim(sim: &Simulation) -> Result<Self, String> {
        Self::parse(&sim.domain_checkpoint_bytes())
    }

    fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_CHECKPOINT || bytes.get(..14) != Some(b"SH:C03:STATE\x01\x00") {
            return Err("checkpoint.layout".into());
        }
        let mut r = Reader { bytes, at: 98 };
        // Header is 98 bytes; scenario has wake, weights, minimums, recipes.
        r.skip(8 + 25 * 8 + 5 * 8)?;
        r.list_skip(49)?;
        let mut actors = Vec::new();
        for _ in 0..r.count(131)? {
            let id = r.u32()?;
            let enabled = r.u8()?;
            let mode = r.u8()?;
            let next_wake = r.u64()?;
            let mut needs = [0; 5];
            for n in &mut needs {
                *n = r.u32()?;
            }
            r.skip(6 * 8)?;
            actors.push(ActorView {
                id,
                enabled,
                mode,
                next_wake,
                needs,
                home: r.u64()?,
                task: r.u64()?,
                locus: r.u8()?,
                site: r.u64()?,
                x: r.u32()?,
                y: r.u32()?,
                edge: r.u64()?,
                traversed: r.u64()?,
            });
        }
        let mut sites = Vec::new();
        for _ in 0..r.count(9)? {
            sites.push(r.u64()?);
            r.skip(1)?;
        }
        r.list_skip(9)?; // holders
        let mut edges = Vec::new();
        for _ in 0..r.count(32)? {
            edges.push((r.u64()?, r.u64()?, r.u64()?, r.u64()?));
        }
        let grid_site = r.u64()?;
        let w = r.u32()? as usize;
        let h = r.u32()? as usize;
        let passable = r
            .take(w.checked_mul(h).ok_or("checkpoint.bounds")?)?
            .to_vec();
        let grid = GridView {
            site: grid_site,
            width: w as u32,
            height: h as u32,
            passable,
        };
        let mut stocks = Vec::new();
        for _ in 0..r.count(34)? {
            stocks.push(StockView {
                id: r.u64()?,
                definition: r.u64()?,
                class: r.u8()?,
                quantity: r.u64()?,
                owner_kind: r.u8()?,
                owner: r.u64()?,
            });
        }
        r.list_skip(53)?; // reservations
        let mut tasks = Vec::new();
        for _ in 0..r.count(104)? {
            let id = r.u64()?;
            let actor = r.u32()?;
            let goal = r.u8()?;
            r.skip(8)?; // recipe
            let stage = r.u8()?;
            r.skip(1 + 1 + 8 + 4 + 4 + 24)?; // interruption, target, progress/timing
            r.list_skip(40)?; // completion records
            r.skip(8)?; // path index
            for _ in 0..r.count(2)? {
                match r.u8()? {
                    1 => r.skip(8)?,
                    2 => r.skip(1)?,
                    _ => return Err("checkpoint.path".into()),
                }
            }
            r.list_skip(8)?;
            r.list_skip(35)?;
            tasks.push(TaskView {
                id,
                actor,
                goal,
                stage,
            });
        }
        Ok(Self {
            actors,
            sites,
            edges,
            grid,
            stocks,
            tasks,
        })
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.at.checked_add(n).ok_or("checkpoint.bounds")?;
        let data = self.bytes.get(self.at..end).ok_or("checkpoint.truncated")?;
        self.at = end;
        Ok(data)
    }
    fn skip(&mut self, n: usize) -> Result<(), String> {
        self.take(n).map(|_| ())
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn count(&mut self, width: usize) -> Result<usize, String> {
        let count = usize::try_from(self.u64()?).map_err(|_| "checkpoint.bounds")?;
        if count > (self.bytes.len().saturating_sub(self.at)) / width {
            return Err("checkpoint.bounds".into());
        }
        Ok(count)
    }
    fn list_skip(&mut self, width: usize) -> Result<(), String> {
        let count = self.count(width)?;
        self.skip(count.checked_mul(width).ok_or("checkpoint.bounds")?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn c04_t02_projection_bounds() {
        let bytes = crate::DEMO;
        for n in 0..bytes.len() {
            assert!(View::validated(&bytes[..n]).is_err());
        }
        let mut hostile = bytes.to_vec();
        hostile[403..411].fill(255);
        assert!(View::parse(&hostile).is_err());
        assert!(View::validated(&hostile).is_err());
        let mut unsupported = bytes.to_vec();
        unsupported[12] = 2;
        assert!(View::parse(&unsupported).is_err());
        assert!(View::validated(&vec![0; MAX_CHECKPOINT + 1]).is_err());
    }
}
