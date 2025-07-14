use std::collections::VecDeque;
use std::mem;
use bevy::prelude::Resource;
use rand_xoshiro::SplitMix64;
use rand::{Rng, SeedableRng};

static EMPTY_CELL: Cell = Cell {
    species: Species::Empty,
    ra: 0,
    rb: 0,
    clock: 0,
};

#[derive(Resource)]
pub struct CellGrid {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) cells: Vec<Cell>,
    undo_stack: VecDeque<Vec<Cell>>,
    pub(crate) winds: Vec<Wind>,
    pub(crate) burns: Vec<Wind>,
    generation: u8,
    rng: SplitMix64,
}


impl CellGrid {
    pub fn reset(&mut self) {
        for x in 0..self.width {
            for y in 0..self.height {
                let idx = self.get_index(x, y);
                self.cells[idx] = EMPTY_CELL;
            }
        }
    }
    pub fn tick(&mut self) {
        // let mut next = self.cells.clone();
        // let dx = self.winds[(self.width * self.height / 2) as usize].dx;
        // let js: JsValue = (dx).into();
        // console::log_2(&"dx: ".into(), &js);

        for x in 0..self.width {
            for y in 0..self.height {
                let cell = self.get_cell(x, y);
                let wind = self.get_wind(x, y);
                CellGrid::blow_wind(
                    cell,
                    wind,
                    SandApi {
                        universe: self,
                        x,
                        y,
                    },
                )
            }
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation  == 255 {
            self.generation /=2;
        }
        for x in 0..self.width {
            let scanx = if self.generation % 2 == 0 {
                self.width - (1 + x)
            } else {
                x
            };

            for y in 0..self.height {
                let idx = self.get_index(scanx, y);
                let cell = self.get_cell(scanx, y);

                self.burns[idx] = Wind {
                    dx: 0,
                    dy: 0,
                    pressure: 0,
                    density: 0,
                };
                CellGrid::update_cell(
                    cell,
                    SandApi {
                        universe: self,
                        x: scanx,
                        y,
                    },
                );
            }
        }

        self.generation = self.generation.wrapping_add(1);
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn cells(&self) -> *const Cell {
        self.cells.as_ptr()
    }

    pub fn winds(&self) -> *const Wind {
        self.winds.as_ptr()
    }

    pub fn burns(&self) -> *const Wind {
        self.burns.as_ptr()
    }
    pub fn paint(&mut self, x: i32, y: i32, size: i32, species: Species) {
        let size = size;
        let radius: f64 = (size as f64) / 2.0;

        let floor = (radius + 1.0) as i32;
        let ciel = (radius + 1.5) as i32;

        for dx in -floor..ciel {
            for dy in -floor..ciel {
                if (((dx * dx) + (dy * dy)) as f64) > (radius * radius) {
                    continue;
                };
                let px = x + dx;
                let py = y + dy;
                let i = self.get_index(px, py);

                if px < 0 || px > self.width - 1 || py < 0 || py > self.height - 1 {
                    continue;
                }
                if self.get_cell(px, py).species == Species::Empty || species == Species::Empty {
                    self.cells[i] = Cell {
                        species: species,
                        ra: 60
                            + (size as u8)
                            + (self.rng.gen::<f32>() * 30.) as u8
                            + ((self.generation % 127) as i8 - 60).abs() as u8,
                        rb: 0,
                        clock: self.generation,
                    }
                }
            }
        }
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push_front(self.cells.clone());
        self.undo_stack.truncate(50);
    }

    pub fn pop_undo(&mut self) {
        let old_state = self.undo_stack.pop_front();
        match old_state {
            Some(state) => self.cells = state,
            None => (),
        };
    }

    pub fn flush_undos(&mut self) {
        self.undo_stack.clear();
    }

    pub fn new(width: i32, height: i32) -> CellGrid {
        let cells = (0..width * height).map(|_i| EMPTY_CELL).collect();
        let winds: Vec<Wind> = (0..width * height)
            .map(|_i| Wind {
                dx: 0,
                dy: 0,
                pressure: 0,
                density: 0,
            })
            .collect();

        let burns: Vec<Wind> = (0..width * height)
            .map(|_i| Wind {
                dx: 0,
                dy: 0,
                pressure: 0,
                density: 0,
            })
            .collect();
        let rng: SplitMix64 = SeedableRng::seed_from_u64(0x734f6b89de5f83cc);
        CellGrid {
            width,
            height,
            cells,
            undo_stack: VecDeque::with_capacity(50),
            burns,
            winds,
            generation: 0,
            rng,
        }
    }
}
impl CellGrid {
fn get_index(&self, x: i32, y: i32) -> usize {
    (x * self.height + y) as usize
}

fn get_cell(&self, x: i32, y: i32) -> Cell {
    let i = self.get_index(x, y);
    return self.cells[i];
}
pub fn get_x_y(&self,i:i32)->(i32,i32){
    (i / self.height,i % self.height)
}

fn get_wind(&self, x: i32, y: i32) -> Wind {
    let i = self.get_index(x, y);
    return self.winds[i];
}

fn blow_wind(cell: Cell, wind: Wind, mut api: SandApi) {
    if cell.clock > api.universe.generation && cell.clock - api.universe.generation == 1 {
        return;
    }
    if cell.species == Species::Empty {
        return;
    }
    let mut dx = 0;
    let mut dy = 0;

    let threshold = match cell.species {
        Species::Empty => 500,
        Species::Wall => 500,
        Species::Cloner => 500,

        Species::Stone => 70,
        Species::Wood => 70,

        Species::Plant => 60,
        Species::Lava => 60,
        Species::Ice => 60,

        Species::Fungus => 54,

        Species::Oil => 50,

        // Intentionally left out and covered by the default case
        // Species::Water => 40,
        // Species::Acid => 40,
        Species::Seed => 35,

        Species::Sand => 30,
        Species::Mite => 30,
        Species::Rocket => 30,

        Species::Dust => 10,
        Species::Fire => 5,
        Species::Gas => 5,
        /*
         Some hacked species values exist outside of the enum values.
         Making sure the default case is emitted allows "BELP" to have a defined wind threshold.
         Originally, threshold was a hardcoded value, so this preserves that original glitch behavior.
         See: https://sandspiel.club/#eMlYGC52XIto0NM1WjaJ
        */
        _ => 40,
    };

    let wx = (wind.dy as i32) - 126;
    let wy = (wind.dx as i32) - 126;

    if wx > threshold {
        dx = 1;
    }
    if wy > threshold {
        dy = 1;
    }
    if wx < -threshold {
        dx = -1;
    }
    if wy < -threshold {
        dy = -1;
    }
    if (dx != 0 || dy != 0) && api.get(dx, dy).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        if dy == -1
            && api.get(dx, -2).species == Species::Empty
            && (cell.species == Species::Sand
            || cell.species == Species::Water
            || cell.species == Species::Lava
            || cell.species == Species::Acid
            || cell.species == Species::Mite
            || cell.species == Species::Dust
            || cell.species == Species::Oil
            || cell.species == Species::Rocket)
        {
            dy = -2;
        }
        api.set(dx, dy, cell);
        return;
    }
}
fn update_cell(cell: Cell, api: SandApi) {
    if cell.clock > api.universe.generation && cell.clock - api.universe.generation == 1 {
        return;
    }

    cell.update(api);
}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub(crate) species: Species,
    pub(crate) ra: u8,
    pub(crate) rb: u8,
    pub(crate) clock: u8,
}
// Cell 的方法：
// new：这是一个构造函数，创建一个新的 Cell 实例。它会基于物种（species）和随机生成的数值来初始化 ra 和 rb 属性。
// update：调用细胞的 species 更新方法来改变细胞的状态。这个方法通过 SandApi（API 代理）来执行物种的更新逻辑。
impl Cell {
    pub fn new(species: Species) -> Cell {
        Cell {
            species: species,
            ra: 100 + (rand::random::<f32>() * 50.) as u8,
            rb: 0,
            clock: 0,
        }
    }
    pub fn update(&self, api: SandApi) {
        self.species.update(*self, api);
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wind {
    pub(crate) dx: u8,
    pub(crate) dy: u8,
    pub(crate) pressure: u8,
    pub(crate) density: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash,Default)]
#[repr(u8)]
pub enum Species {
    #[default]
    Empty = 0,
    Wall = 1,
    Sand = 2,
    Water = 3,
    // X = 21,
    Stone = 13,
    Ice = 9,
    Gas = 4,
    Cloner = 5,
    // Sink = 10,
    Mite = 15,
    Wood = 7,
    Plant = 11,
    Fungus = 18,
    Seed = 19,
    Fire = 6,
    Lava = 8,
    Acid = 12,
    Dust = 14,
    Oil = 16,
    Rocket = 17,
}
impl Species {
    // Species::update 方法是一个分发器，根据不同的物种类型调用不同的更新函数。每个物种的行为是由其对应的 update_* 方法决定的。
    pub fn update(&self, cell: Cell, api: SandApi) {
        match self {
            Species::Empty => {}
            Species::Wall => {}
            Species::Sand => update_sand(cell, api),
            Species::Dust => update_dust(cell, api),
            Species::Water => update_water(cell, api),
            Species::Stone => update_stone(cell, api),
            Species::Gas => update_gas(cell, api),
            Species::Cloner => update_cloner(cell, api),
            Species::Rocket => update_rocket(cell, api),
            Species::Fire => update_fire(cell, api),
            Species::Wood => update_wood(cell, api),
            Species::Lava => update_lava(cell, api),
            Species::Ice => update_ice(cell, api),
            // Species::Snow => update_ice(cell, api),
            //lightning
            // Species::Sink => update_sink(cell, api),
            Species::Plant => update_plant(cell, api),
            Species::Acid => update_acid(cell, api),
            Species::Mite => update_mite(cell, api),
            Species::Oil => update_oil(cell, api),
            Species::Fungus => update_fungus(cell, api),
            Species::Seed => update_seed(cell, api),
            // Species::X => update_x(cell, api),
        }
    }
}

pub struct SandApi<'a> {
    x: i32,
    y: i32,
    universe: &'a mut CellGrid,
}

impl<'a> SandApi<'a> {
    pub fn get(&mut self, dx: i32, dy: i32) -> Cell {
        if dx > 2 || dx < -2 || dy > 2 || dy < -2 {
            panic!("oob set");
        }
        let nx = self.x + dx;
        let ny = self.y + dy;
        if nx < 0 || nx > self.universe.width - 1 || ny < 0 || ny > self.universe.height - 1 {
            return Cell {
                species: Species::Wall,
                ra: 0,
                rb: 0,
                clock: self.universe.generation,
            };
        }
        self.universe.get_cell(nx, ny)
    }
    pub fn set(&mut self, dx: i32, dy: i32, v: Cell) {
        if dx > 2 || dx < -2 || dy > 2 || dy < -2 {
            panic!("oob set");
        }
        let nx = self.x + dx;
        let ny = self.y + dy;

        if nx < 0 || nx > self.universe.width - 1 || ny < 0 || ny > self.universe.height - 1 {
            return;
        }
        let i = self.universe.get_index(nx, ny);
        // v.clock += 1;
        self.universe.cells[i] = v;
        self.universe.cells[i].clock = self.universe.generation.wrapping_add(1);
    }
    pub fn get_fluid(&mut self) -> Wind {
        let idx = self.universe.get_index(self.x, self.y);

        self.universe.winds[idx]
    }
    pub fn set_fluid(&mut self, v: Wind) {
        let idx = self.universe.get_index(self.x, self.y);

        self.universe.burns[idx] = v;
    }

    pub fn rand_int(&mut self, n: i32) -> i32 {
        self.universe.rng.gen_range(0..n)
    }

    pub fn once_in(&mut self, n: i32) -> bool {
        self.rand_int(n) == 0
    }
    pub fn rand_dir(&mut self) -> i32 {
        let i = self.rand_int(1000);
        (i % 3) - 1
    }
    pub fn rand_dir_2(&mut self) -> i32 {
        let i = self.rand_int(1000);
        if (i % 2) == 0 {
            -1
        } else {
            1
        }
    }

    pub fn rand_vec(&mut self) -> (i32, i32) {
        let i = self.rand_int(2000);
        match i % 9 {
            0 => (1, 1),
            1 => (1, 0),
            2 => (1, -1),
            3 => (0, -1),
            4 => (-1, -1),
            5 => (-1, 0),
            6 => (-1, 1),
            7 => (0, 1),
            _ => (0, 0),
        }
    }

    pub fn rand_vec_8(&mut self) -> (i32, i32) {
        let i = self.rand_int(8);
        match i {
            0 => (1, 1),
            1 => (1, 0),
            2 => (1, -1),
            3 => (0, -1),
            4 => (-1, -1),
            5 => (-1, 0),
            6 => (-1, 1),
            _ => (0, 1),
        }
    }
}

// update_sand 方法处理沙子的行为。沙子会根据周围环境进行下落：
//
// 如果下方是空的，沙子会下落。
// 如果旁边是空的，则沙子会向旁边移动。
// 如果周围有水、气体、油或酸，沙子也会交换位置。

// 沙子的更新逻辑是根据其周围的细胞状态来决定的。
pub fn update_sand(cell: Cell, mut api: SandApi) {
    let dx = api.rand_dir_2();

    let nbr = api.get(0, 1);
    if nbr.species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(0, 1, cell);
    } else if api.get(dx, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(dx, 1, cell);
    } else if nbr.species == Species::Water
        || nbr.species == Species::Gas
        || nbr.species == Species::Oil
        || nbr.species == Species::Acid
    {
        api.set(0, 0, nbr);
        api.set(0, 1, cell);
    } else {
        api.set(0, 0, cell);
    }
}

// update_dust 方法描述了尘土的行为：
//
// 如果流体的压力大于 120，尘土会变为火，并生成一个风流体（Wind）。
// 否则，尘土会与周围的空白或水的细胞交换位置，或者保持原位
pub fn update_dust(cell: Cell, mut api: SandApi) {
    let dx = api.rand_dir();
    let fluid = api.get_fluid();

    if fluid.pressure > 120 {
        api.set(
            0,
            0,
            Cell {
                species: Species::Fire,
                ra: (150 + (cell.ra / 10)) as u8,
                rb: 0,
                clock: 0,
            },
        );
        api.set_fluid(Wind {
            dx: 0,
            dy: 0,
            pressure: 80,
            density: 5,
        });
        return;
    }

    let nbr = api.get(0, 1);
    if nbr.species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(0, 1, cell);
    } else if nbr.species == Species::Water {
        api.set(0, 0, nbr);
        api.set(0, 1, cell);
    } else if api.get(dx, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(dx, 1, cell);
    } else {
        api.set(0, 0, cell);
    }
}

// update_stone 方法描述了石头的行为：
//
// 如果石头的两侧都有石头，石头不会移动。
// 如果流体的压力大于 120，石头可能会变成沙子。
// 否则，石头会尝试向下移动或与周围的细胞交换。
pub fn update_stone(cell: Cell, mut api: SandApi) {
    if api.get(-1, -1).species == Species::Stone && api.get(1, -1).species == Species::Stone {
        return;
    }
    let fluid = api.get_fluid();

    if fluid.pressure > 120 && api.rand_int(1) == 0 {
        api.set(
            0,
            0,
            Cell {
                species: Species::Sand,
                ra: cell.ra,
                rb: 0,
                clock: 0,
            },
        );
        return;
    }

    let nbr = api.get(0, 1);
    let nbr_species = nbr.species;
    if nbr_species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(0, 1, cell);
    } else if nbr_species == Species::Water
        || nbr_species == Species::Gas
        || nbr_species == Species::Oil
        || nbr_species == Species::Acid
    {
        api.set(0, 0, nbr);
        api.set(0, 1, cell);
    } else {
        api.set(0, 0, cell);
    }
}

// 在一个细胞自动机的模拟中处理水的行为，可能是用来模拟沙盒游戏或者物理引擎中的流体行为。它通过不同的条件和随机行为来操控当前水的细胞及其邻近的细胞。
// cell: 当前的水细胞。  api: 一个引用 SandApi 的对象，提供了随机数生成和操作邻近细胞的方法。
pub fn update_water(cell: Cell, mut api: SandApi) {
    let mut dx = api.rand_dir();  // 随机方向
    let below = api.get(0, 1);    // 获取下方细胞
    let dx1 = api.get(dx, 1);     // 获取斜下方细胞
    // let mut dx0 = api.get(dx, 0);
    //fall down
    //1. 下落（重力效果）：
    //  如果下方的细胞为空或含有油，水就会下落到下方的空细胞。并且有一定概率会随机改变水流的方向 (ra)，然后更新下方的细胞状态。

    // 函数首先检查下方的细胞 (below) 是否为空或含有油 (Species::Empty || Species::Oil)。
    // 如果是的话，它将当前水细胞移动到下方，并在一定概率下随机改变水的方向 (ra)，模拟水流的随机性。
    if below.species == Species::Empty || below.species == Species::Oil {
        //  移动到下方
        api.set(0, 0, below);
        let mut ra = cell.ra;
        // 每20次随机改变方向
        if api.once_in(20) {
            //randomize direction when falling sometimes

            ra = 100 + api.rand_int(50) as u8;
        }
        // 更新下方细胞的状态
        api.set(0, 1, Cell { ra, ..cell });

        return;
    } else if dx1.species == Species::Empty || dx1.species == Species::Oil {
        // 斜向下落：
        //如果水流方向斜下方的细胞为空或含有油，水会沿斜线下落到该细胞。
        // 如果斜对角方向的细胞（dx1）为空或含有油，水会沿斜线下落到该位置。
        //fall diagonally
        api.set(0, 0, dx1);  // 移动到斜下方
        api.set(dx, 1, cell); // 更新当前位置
        return;
    } else if api.get(-dx, 1).species == Species::Empty {
        // 如果水流方向反方向的细胞为空，水就会向反方向移动。
        api.set(0, 0, EMPTY_CELL);  // 清空当前位置
        api.set(-dx, 1, cell);  // 将水移动到反方向的下方
        return;
    }


    let left = cell.ra % 2 == 0;  // 判断当前水是否在左侧（基于 ra）
    dx = if left { 1 } else { -1 };  // 根据 ra 确定方向
    let dx0 = api.get(dx, 0);  // 获取水流方向上的细胞
    let dxd = api.get(dx * 2, 0);  // 获取更远的细胞

    if dx0.species == Species::Empty && dxd.species == Species::Empty {
        // scoot double
        // 双重滑动：
        //  如果当前方向和更远的位置都为空，水会移动到更远的空位置，模拟水流的扩展。

        // 如果发现两格空的细胞（水平方向或垂直方向），水会向更远的一个位置移动，
        // 这样可以使水扩散得更广，并且可能会改变 ra 属性（这可能与水的行为或颜色相关）。
        api.set(0, 0, dxd);  // 移动到更远的空位置
        api.set(2 * dx, 0, Cell { rb: 6, ..cell });  // 设置新位置的状态
        let (dx, dy) = api.rand_vec_8();  // 随机获取周围邻居
        let nbr = api.get(dx, dy);

        // spread opinion
        // 扩散：
        //
        // 如果水附近有其他水细胞，函数会检查它们的 ra 值（可能是颜色或状态指示符）。
        // 如果它们不同，水会通过复制另一个水细胞的 ra 值来进行“扩散”。

        if nbr.species == Species::Water {
            if nbr.ra % 2 != cell.ra % 2 {
                api.set(
                    dx,
                    dy,
                    Cell {
                        ra: cell.ra,
                        ..cell
                    },
                )
            }
        }
    } else if dx0.species == Species::Empty || dx0.species == Species::Oil {
        // 当前水流方向上的邻居是否为空（Species::Empty）或者含有油   如果是空的或者是油，水就可以流到该位置。
        // 模拟水流在碰到空细胞或油时的行为，并尝试使水与周围的水细胞发生交互，特别是在它们的 ra
        api.set(0, 0, dx0);  // 将当前位置设置为 dx0（可能为空或者油）
        api.set(dx, 0, Cell { rb: 3, ..cell });  // 将水移动到 dx 方向，设置 rb 为 3
        let (dx, dy) = api.rand_vec_8();  // 随机选择一个八个方向中的一个邻居
        let nbr = api.get(dx, dy);  // 获取该邻居细胞

        if nbr.species == Species::Water {  // 如果邻居是水
            if nbr.ra % 2 != cell.ra % 2 {  // 如果邻居的 ra 与当前水的 ra 不同
                api.set(  // 更新邻居的状态，使它的 ra 与当前水的 ra 一致
                          dx,
                          dy,
                          Cell {
                              ra: cell.ra,  // 设置相同的 ra
                              ..cell
                          },
                );
            }
        }
    } else if cell.rb == 0 {
        // 碰撞：
        // 如果当前水的“碰撞性” (rb) 为零，水会检查反方向的邻居是否为空，如果为空，水会“碰撞”并更新位置。
        //
        // 如果 rb（可能是“碰撞”或交互计数器）为零，函数会检查水的反方向邻近细胞（-dx）是否为空，如果为空，水会“碰撞”并调整其 ra 值。
        // 减少“碰撞性”：
        //
        // 如果 rb 值大于零，水会减少 rb 值，这意味着水的“碰撞性”降低，更容易发生下一次的碰撞或交互。
        if api.get(-dx, 0).species == Species::Empty {
            // bump
            api.set(
                0,
                0,
                Cell {
                    ra: ((cell.ra as i32) + dx) as u8,
                    ..cell
                },
            );
        }
    } else {
        // 减少“碰撞性”：
        // 如果水的 rb 值大于零，表示水变得更易发生碰撞，水的 rb 值减少。
        // become less certain (more bumpable)
        api.set(
            0,
            0,
            Cell {
                rb: cell.rb - 1,
                ..cell
            },
        );
    }
    // if api.once_in(8) {
    //     let (dx, dy) = api.rand_vec_8();
    //     let nbr = api.get(dx, dy);
    //     if nbr.species == Species::Water {
    //         if nbr.ra % 2 != cell.ra % 2 {
    //             api.set(0, 0, Cell { ra: nbr.ra, ..cell })
    //         }
    //     }
    // }

    // let (dx, dy) = api.rand_vec_8();
    // let nbr = api.get(dx, dy);
    // if nbr.species == Species::Water {
    //     if nbr.ra % 2 != cell.ra % 2 && api.once_in(2) {
    //         api.set(0, 0, Cell { ra: nbr.ra, ..cell })
    //     }
    // }

    // {

    // if api.get(-dx, 0).species == Species::Empty {
    //     api.set(0, 0, EMPTY_CELL);
    //     api.set(-dx, 0, cell);
    // }
}

// 它通过检查油的状态、周围细胞的状态以及与其他物质（如水、火、熔岩等）的交互来决定油的变化与移动

// update_oil 函数模拟了油在不同条件下的行为，包括：
//
// 油的流动和扩散。
// 油与火、熔岩、油、以及水的交互。
// 油的 rb 值（可能代表油的粘度或状态）变化。
// 油在与空白细胞交互时的行为。
pub fn update_oil(cell: Cell, mut api: SandApi) {
    let rb = cell.rb;  // 获取油的 rb（可能是粘度或流动性）值
    let (dx, dy) = api.rand_vec();  // 获取一个随机方向，dx 和 dy 代表了油的移动方向

    let mut new_cell = cell;  // 创建一个新的 cell，初始化为当前的 cell
    let nbr = api.get(dx, dy);  // 获取油的目标邻居（在 dx, dy 方向上的细胞）

    // 1 火和熔岩与油的交互：
    //
    // 如果油的 rb 为 0，且周围有火或熔岩，油会变成新的油，rb 设置为 50，这可能意味着油被加热或改变了粘度。
    // 如果油遇到另一个油，且该油的 rb 值在 1 到 20 之间，油的 rb 会变为 50，这可能代表两种油的混合或者油的状态发生了改变。
    if rb == 0 && nbr.species == Species::Fire
        || nbr.species == Species::Lava
        || (nbr.species == Species::Oil && nbr.rb > 1 && nbr.rb < 20)
    {
        // 如果符合条件，将油变为新的油，且设置 rb 为 50，代表油变得更加粘稠或具有特定特性
        new_cell = Cell {
            species: Species::Oil,
            ra: cell.ra,
            rb: 50,  // 可能代表油的粘稠度增加
            clock: 0,
        };
    }
    // 2 油的流动和粘度变化：
    //
    // 当油的 rb 值大于 1 时，油的 rb 值会递减，表示油的粘度逐渐变小，变得更容易流动。
    // 如果油的 rb 值不能被 4 整除，并且周围是空的、不是水的地方，则油会引发火灾（将该位置设置为火）。

    // 如果油的 rb 值大于 1，油的状态会有所改变
    if rb > 1 {
        new_cell = Cell {
            species: Species::Oil,
            ra: cell.ra,
            rb: rb - 1,  // 使 rb 值减小，可能代表油逐渐变得不那么粘稠
            clock: 0,
        };

        // 设置流体的特性，模拟油的流动或蒸发
        api.set_fluid(Wind {
            dx: 0,
            dy: 10,
            pressure: 10,
            density: 180,
        });

        // 如果油的 rb 值不能被 4 整除，且邻居为空或不是水，可能产生火
        if rb % 4 != 0 && nbr.species == Species::Empty && nbr.species != Species::Water {
            let ra = 20 + api.rand_int(30) as u8;  // 生成一个随机的火的 ra 值
            api.set(
                dx,
                dy,
                Cell {
                    species: Species::Fire,  // 设置邻居为火
                    ra,
                    rb: 0,
                    clock: 0,
                },
            );
        }
        // 3 油与水的交互：
        //
        // 如果油遇到水，油的 rb 会变为 0，表示油被水冲散，油的流动性可能变为水一样，或者油和水发生了混合。

        // 如果邻居是水，则油的 rb 值变为 0，表示油被水冲走
        if nbr.species == Species::Water {
            new_cell = Cell {
                species: Species::Oil,
                ra: 50,
                rb: 0,  // 设置油的 rb 为 0，可能表示油被水冲走了
                clock: 0,
            };
        }
    } else if rb == 1 {  // 如果油的 rb 为 1，则油的状态更新为空
        //4 油的转变和清空：
        //
        // 如果油的 rb 为 1，油将被设置为空，表示油已经消散或者流动完。
        api.set(
            0,
            0,
            Cell {
                species: Species::Empty,
                ra: cell.ra,
                rb: 90,  // 设置新的 rb 值为 90
                clock: 0,
            },
        );
        return;
    }
    // 5 油的移动：
    //
    // 如果油下方或周围的邻居是空的，油会向这些空白位置流动。油的流动遵循从当前位置（0, 0）向下、斜下、左下、右下等方向寻找空位置的顺序。
    // 如果所有周围位置都不是空的，油会停留在当前位置。

    // 油的移动逻辑：如果下方或其它相邻位置是空的，油会流到该位置
    if api.get(0, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);  // 清空当前位置
        api.set(0, 1, new_cell);  // 将油放置到下方
    } else if api.get(dx, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);  // 清空当前位置
        api.set(dx, 1, new_cell);  // 将油放置到斜下方
    } else if api.get(-dx, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);  // 清空当前位置
        api.set(-dx, 1, new_cell);  // 将油放置到反方向的下方
    } else if api.get(dx, 0).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);  // 清空当前位置
        api.set(dx, 0, new_cell);  // 将油放置到水平方向
    } else if api.get(-dx, 0).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);  // 清空当前位置
        api.set(-dx, 0, new_cell);  // 将油放置到反方向的水平方向
    } else {
        // 如果没有空位置，保持当前位置不变
        api.set(0, 0, new_cell);
    }
}

// 模拟气体（Species::Gas）的行为。函数根据气体的 rb 值（可能代表气体的浓度、压力或体积）以及与周围细胞的交互来更新气体的状态
// rb 值被用来表示气体的浓度、粒子数或类似的物理量。
// 如果 rb 为 0，则气体的状态会发生改变，rb 设置为 5，表示气体浓度或“压力”的增加。
// 气体在扩散过程中会逐渐减少 rb 值，模拟气体粒子从一个位置扩散到相邻位置。

pub fn update_gas(cell: Cell, mut api: SandApi) {
    let (dx, dy) = api.rand_vec();  // 获取一个随机方向，dx 和 dy 代表气体的移动方向

    let nbr = api.get(dx, dy);  // 获取该方向上的邻居细胞
    // api.set_fluid(Wind {
    //     dx: 0,
    //     dy: 0,
    //     pressure: 5,
    //     density: 0,
    // });
    // 如果当前气体的 rb 值为 0，这可能表示气体处于某种初始状态（如“分子”状态），则将其 rb 设置为 5，
    // 表示气体变得更加“浓密”或具有某种其他特性（可能是气体的压力或浓度）。
    if cell.rb == 0 {
        api.set(0, 0, Cell { rb: 5, ..cell });
    }
    // 1 气体的扩散：
    //
    // 当气体的 rb 小于 3 时，气体会作为单个分子（或者表示气体粒子）向邻居细胞移动。
    // 当气体的 rb 大于等于 3 时，气体会分散（rb 值减小），并且气体的一部分被移动到邻居位置。

    // 如果邻居是空的（Species::Empty），则气体会向空细胞移动。
    // 如果气体的 rb 小于 3（可能表示气体为单个分子），则将气体从当前位置移动到邻居位置。
    // 如果气体的 rb 大于等于 3，则气体的 rb 会减小（通过设置当前位置的 rb 为 1，表示气体分散），并将剩余的气体移动到邻居位置。
    if nbr.species == Species::Empty {
        if cell.rb < 3 {
            //single molecule
            api.set(0, 0, EMPTY_CELL);
            api.set(dx, dy, cell);
        } else {
            api.set(0, 0, Cell { rb: 1, ..cell });
            api.set(
                dx,
                dy,
                Cell {
                    rb: cell.rb - 1,
                    ..cell
                },
            );
        }
    } else if (dx != 0 || dy != 0) && nbr.species == Species::Gas && nbr.rb < 4 {
        // 2  气体与其他气体的交互： 气体的合并
        //
        // 当邻居是气体时，如果邻居的 rb 小于 4，当前气体会与邻居气体的 rb 合并。这代表了气体的“聚集”或“扩散”现象。

        // 如果邻居是气体（Species::Gas），并且邻居的 rb 值小于 4（表示邻居气体的浓度较低），则气体会与邻居气体发生合并。
        // 当前气体的 rb 值会与邻居气体的 rb 值相加，表示气体浓度的增加。
        // 然后，当前位置被清空，气体被放置到邻居的位置。
        // if (cell.rb < 2) {
        api.set(0, 0, EMPTY_CELL);
        // }
        api.set(
            dx,
            dy,
            Cell {
                rb: nbr.rb + cell.rb,
                ..cell
            },
        );
    }
}
// pub fn update_x(cell: Cell, mut api: SandApi) {
//     let (dx, dy) = api.rand_vec_8();

//     let nbr = api.get(dx, dy);

//     if nbr.species == Species::X {
//         let opposite = api.get(-dx, -dy);
//         if opposite.species == Species::Empty {
//             api.set(0, 0, EMPTY_CELL);
//             api.set(-dx, -dy, cell);
//         }
//     }
// }

// “克隆体”（Cloner）的物质在模拟环境中的行为。克隆体根据周围的环境不断复制自己或尝试克隆其他物质。
// 1 克隆体会根据周围的细胞类型和状态，克隆出新的细胞。
// 2 克隆体会根据当前的 rb 值来决定是否克隆其他物质（如水、沙子、油等），或者自身继续克隆。
// 3 如果 rb 为 0，则克隆体会选择一个相邻的非空白细胞，克隆该物质。
// 4 如果 rb 不为 0，则克隆体会尝试在周围的空白细胞中创建一个新的克隆体。

// 多样化的克隆条件：可以根据 generation 或 ra 值调整克隆体的克隆行为，使其更加有趣和复杂。
// 克隆体之间的竞争或互动：可以加入克隆体之间的互动规则，比如克隆体相互之间的冲突或竞争。
pub fn update_cloner(cell: Cell, mut api: SandApi) {
    let mut clone_species = unsafe { mem::transmute(cell.rb as u8) };  // 将 `cell.rb` 转换为物种类型
    let g = api.universe.generation;  // 获取当前的宇宙代数
    // 这部分代码是用来遍历克隆体周围的 3x3 区域（包括当前位置）。
    // dx 和 dy 分别代表 x 和 y 方向上的偏移，范围从 -1 到 1。
    for dx in [-1, 0, 1].iter().cloned() {
        for dy in [-1, 0, 1].iter().cloned() {
            // 1.克隆目标选择：
            //
            // 如果 cell.rb == 0，克隆体会选择一个相邻的非空白细胞进行克隆，克隆体的 rb 值将被设置为新克隆物质的物种类型。

            // 如果 cell.rb 为 0（即克隆体未被激活或正在选择克隆目标）：
            // 获取相邻细胞的物种（nbr_species）。
            // 如果相邻细胞的物种不是空的（Species::Empty）、克隆体（Species::Cloner）或墙（Species::Wall），则将该细胞的物种类型赋给 clone_species，表示克隆体将克隆此物种。
            // 然后，克隆体（cell）会被设置为一个新细胞，ra 设置为 200（可能是克隆体的属性，或表示克隆体的激活状态），并且其 rb 值变为新克隆的物种类型。
            if cell.rb == 0 {
                let nbr_species = api.get(dx, dy).species;
                if nbr_species != Species::Empty
                    && nbr_species != Species::Cloner
                    && nbr_species != Species::Wall
                {
                    clone_species = nbr_species;
                    api.set(
                        0,
                        0,
                        Cell {
                            species: cell.species,
                            ra: 200,
                            rb: clone_species as u8,
                            clock: 0,
                        },
                    );

                    break;
                }
            } else {
                // 2. 克隆体的扩散：
                //
                // 如果 cell.rb 不为 0，克隆体会尝试在周围的空白细胞中创建新的克隆体。这个过程是随机的，且具有一定的扩展性。
                // 3 克隆体的 ra 值调整：
                //
                // 4 克隆体的 ra 值会根据当前代数（g）以及随机值进行调整，可能用来表示克隆体的某种特性或状态。
                // 克隆体的限制条件：
                //
                // 克隆体不会克隆空白细胞、墙、或者其他克隆体；它只会克隆非空的物质，且具有一定的随机性和扩展能力

                // 如果 cell.rb 不为 0（即克隆体已经激活或正在克隆）：
                // 克隆体会尝试在周围的空白细胞（Species::Empty）中创建一个新的克隆体。这个概率通过 rand_int(100) > 90 来控制，即 10% 的几率。
                // 如果选择的位置为空（Species::Empty），则克隆体会将一个新细胞放置在该位置，ra 会根据当前代数（g）和一个随机值来计算，以使得新克隆的细胞在某种程度上具备一定的随机性。
                // 新创建的克隆体的 rb 值被设置为 0，表示它是一个新生的克隆体。
                if api.rand_int(100) > 90 && api.get(dx, dy).species == Species::Empty {
                    let ra = 80 + api.rand_int(30) as u8 + ((g % 127) as i8 - 60).abs() as u8;
                    api.set(
                        dx,
                        dy,
                        Cell {
                            species: clone_species,
                            ra,
                            rb: 0,
                            clock: 0,
                        },
                    );
                    break;
                }
            }
        }
    }
}


// 一个具有复杂行为的“火箭” (Rocket) 物质更新机制，涉及火箭的多个阶段（如初始化、待机、发射、飞行）。
// 其状态通过 ra 和 rb 属性来控制，每个阶段的行为不同，且会根据周围环境的情况进行相应的调整。

// update_rocket 函数通过不同的 ra 值来控制火箭的多个阶段行为，具体如下：
//
// 初始化：火箭开始时没有类型，随机选择一个邻近的非空白细胞并设置其物种。
// 待机/降落：火箭会根据周围环境进行降落或保持原地，遇到合适的环境时下落。
// 发射：火箭从待机状态进入发射状态。
// 飞行：火箭在空中飞行，根据随机方向移动。
// 扩展或熄火：火箭在飞行过程中可能扩展、克隆或者熄火，结束飞行。
// 可能的改进
// 火箭控制：可以根据某些条件调整火箭的速度、方向或行为。
// 火箭与其他物质互动：火箭可以与更多种类的物质互动，比如引发爆炸或碰撞等。

pub fn update_rocket(cell: Cell, mut api: SandApi) {
    // rocket has complicated behavior that is staged piecewise in ra.
    // it would be awesome to diagram the ranges of values and their meaning
    // 火箭的初始化阶段：如果 cell.rb 为 0，表示火箭处于未初始化状态。在这种情况下，火箭被初始化为一个新的细胞，ra 为 0，rb 设置为 100，表示火箭的初始状态。
    // 此时，ra 和 rb 用来表示火箭的不同状态。
    if cell.rb == 0 {
        //initialize
        api.set(
            0,
            0,
            Cell {
                ra: 0,
                rb: 100,
                ..cell
            },
        );
        return;
    }

    // 物种设置阶段
    // 这里根据 cell.rb 的值来确定火箭的物种类型。如果 cell.rb 不为 100，则将 cell.rb 转换为一个物种（Species）。
    // 如果 cell.rb 为 100，则设置为沙子 (Species::Sand)。
    let clone_species = if cell.rb != 100 {
        unsafe { mem::transmute(cell.rb as u8) }
    } else {
        Species::Sand
    };

    // 火箭的行为：未设置类型
    // 如果 cell.rb 为 100，表示火箭尚未设置物种。在此情况下，程序随机选择一个邻近的空白细胞，
    // 获取其物种类型。如果该物种有效（不是空白、火箭、墙或克隆体），
    // 则将火箭的 rb 设置为该物种的类型，并且 ra 设置为 1（表示火箭已设置物种类型）。
    let (sx, sy) = api.rand_vec();
    let sample = api.get(sx, sy);

    if cell.rb == 100 //the type is unset
        && sample.species != Species::Empty
        && sample.species != Species::Rocket
        && sample.species != Species::Wall
        && sample.species != Species::Cloner
    {
        api.set(
            0,
            0,
            Cell {
                ra: 1,
                rb: sample.species as u8, //store the type
                ..cell
            },
        );
        return;
    }
    // 火箭待机（降落）阶段
    // 如果 ra 为 0，表示火箭处于待机或降落状态。
    // 程序随机选择一个方向，并检查当前下方（0, 1）的细胞类型。
    // 如果下方为空白，则火箭将下落到该位置。
    // 如果下方有液体（如水、气体、油、酸等），则火箭也会与该物质交换位置，保持原地。
    // 如果以上条件都不满足，火箭将保持原地。
    let ra = cell.ra;

    if ra == 0 {
        //falling (dormant)
        let dx = api.rand_dir();
        let nbr = api.get(0, 1);
        if nbr.species == Species::Empty {
            api.set(0, 0, EMPTY_CELL);
            api.set(0, 1, cell);
        } else if api.get(dx, 1).species == Species::Empty {
            api.set(0, 0, EMPTY_CELL);
            api.set(dx, 1, cell);
        } else if nbr.species == Species::Water
            || nbr.species == Species::Gas
            || nbr.species == Species::Oil
            || nbr.species == Species::Acid
        {
            api.set(0, 0, nbr);
            api.set(0, 1, cell);
        } else {
            api.set(0, 0, cell);
        }
    } else if ra == 1 {
        // 火箭发射阶段
        // 如果 ra 为 1，表示火箭已启动。此时，火箭的状态更新为 ra = 2，进入飞行阶段。
        //launch
        api.set(0, 0, Cell { ra: 2, ..cell });
    } else if ra == 2 {
        // 火箭飞行阶段
        // 如果 ra 为 2，表示火箭已开始飞行。程序会根据随机的方向生成一个移动向量（dx, dy）。
        // 如果目标位置不是空白，则火箭的飞行方向会反向（dx 和 dy 取反）。
        // 火箭的 ra 值会更新为 100 + join_dy_dx(dx, dy)，这表示火箭的飞行方向和状态。
        let (mut dx, mut dy) = api.rand_vec_8();
        let nbr = api.get(dx, dy);
        if nbr.species != Species::Empty {
            dx *= -1;
            dy *= -1;
        }
        api.set(
            0,
            0,
            Cell {
                ra: 100 + join_dy_dx(dx, dy),
                ..cell
            },
        );
    } else if ra > 50 {
        // 火箭的进一步扩展或熄火
        // 如果 ra 值大于 50，表示火箭正在扩展或飞行到更远的地方。
        // 火箭将根据其 ra 值的差异计算新的飞行方向（通过 split_dy_dx 函数）。
        // 如果目标位置为空白、火或其他火箭，火箭会在当前位置生成克隆物质（clone_species），并继续向新的方向飞行。
        // 如果目标位置不符合条件，则火箭熄火（fizzle），被清空。
        let (dx, dy) = split_dy_dx(cell.ra - 100);

        let nbr = api.get(dx, dy * 2);

        if nbr.species == Species::Empty
            || nbr.species == Species::Fire
            || nbr.species == Species::Rocket
        {
            api.set(0, 0, Cell::new(clone_species));
            api.set(0, dy, Cell::new(clone_species));

            let (ndx, ndy) = match api.rand_int(100) % 5 {
                0 => adjacency_left((dx, dy)),
                1 => adjacency_right((dx, dy)),
                // 2 => adjacency_right((dx, dy)),
                _ => (dx, dy),
            };
            api.set(
                dx,
                dy * 2,
                Cell {
                    ra: 100 + join_dy_dx(ndx, ndy),
                    ..cell
                },
            );
        } else {
            //fizzle
            api.set(0, 0, EMPTY_CELL);
        }
    }
}


// 火焰 (Fire) 物质的更新逻辑，控制火焰的扩散、降解以及与周围环境的互动。具体来说，
// 它涉及火焰的降解过程、与气体或灰尘的相互作用、风力的应用以及火焰与水或其他物质的交互

// 降解：火焰的强度会随着时间和扩散的随机性而降低。每次更新时，火焰的强度会被减小，模拟火焰的衰退过程。
// 扩散：火焰会随机向周围的空白区域或气体、灰尘等物质扩散。如果扩散到的地方是气体或灰尘，则会在该位置生成新的火焰。
// 与水的交互：如果火焰扩散到水的周围，火焰会熄灭。
// 风的影响：风会对火焰的扩散产生影响，每次火焰更新时，都会设置风的压力和密度，模拟火焰受到风力的推动。
// 随机性：火焰的扩散和降解具有很强的随机性，尤其是在与周围物质互动时，每次更新都会基于随机方向来决定火焰的行为。

pub fn update_fire(cell: Cell, mut api: SandApi) {
    // 1. 火焰降解
    // 获取当前火焰细胞的 ra 值（表示火焰的强度或阶段）。
    // 创建一个 degraded 变量，用来表示火焰的降解状态。降解是通过将当前的 ra 值减去一个随机值来实现的，
    // api.rand_dir() 返回一个随机的方向值（可能是 -1、0、1），因此这个变化是有随机性的。
    let ra = cell.ra;
    let mut degraded = cell.clone();
    degraded.ra = ra - (2 + api.rand_dir()) as u8;

    // 2. 随机选择一个方向进行扩散
    // api.rand_vec() 返回一个随机的二维向量 (dx, dy)，用来表示火焰扩散的方向。
    let (dx, dy) = api.rand_vec();

    // 3. 设置风力参数
    // api.set_fluid() 设置风力的影响参数：
    // dx: 0, dy: 150 表示风的方向是沿着 y 轴（向下）。
    // pressure: 1 设置风的压力。
    // density: 120 设置风的密度。
    api.set_fluid(Wind {
        dx: 0,
        dy: 150,
        pressure: 1,
        density: 120,
    });
    // 4. 火焰与气体或灰尘的交互
    // 如果火焰扩散到的地方是气体 (Species::Gas) 或灰尘 (Species::Dust)，则在该位置产生新的火焰细胞。
    // 新的火焰细胞的 ra 值是一个计算结果 (150 + (dx + dy) * 10)，这个值是基于扩散的方向来设置的，意味着火焰会根据其扩散方向产生不同的强度。
    // rb: 0 和 clock: 0 初始化火焰细胞的其他属性。
    // 然后，再次设置风力参数，风的压力和密度有所增加。
    if api.get(dx, dy).species == Species::Gas || api.get(dx, dy).species == Species::Dust {
        api.set(
            dx,
            dy,
            Cell {
                species: Species::Fire,
                ra: (150 + (dx + dy) * 10) as u8,
                rb: 0,
                clock: 0,
            },
        );
        api.set_fluid(Wind {
            dx: 0,
            dy: 0,
            pressure: 80,
            density: 40,
        });
    }
    // 5. 火焰与水或空白区域的交互
    // 如果火焰的强度 ra 小于 5，或者扩散到的地方是水 (Species::Water)，则火焰会被熄灭（设置为空白细胞 EMPTY_CELL）。
    // 如果扩散到的地方是空白 (Species::Empty)，则将当前火焰置为空白，并将降解后的火焰放置到新的位置。
    // 如果扩散到的地方不是空白且也不是水，则将火焰的降解状态放置在当前位置。
    if ra < 5 || api.get(dx, dy).species == Species::Water {
        api.set(0, 0, EMPTY_CELL);
    } else if api.get(dx, dy).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(dx, dy, degraded);
    } else {
        api.set(0, 0, degraded);
    }
}

// 模拟“岩浆”（Lava）在沙盒模拟程序中的行为。它涉及了岩浆与周围环境的交互、流动以及与其他物质的反应（例如与水、火等的反应）
pub fn update_lava(cell: Cell, mut api: SandApi) {
    //    1. 设置风力流体
    // 这行代码设置了风的流动方向和一些物理属性。这里的 dx: 0, dy: 10 表示风的流动方向为向下（沿Y轴方向，单位可能是像素或格子单元）。
    // pressure 为 0，表示风的压力较小，density: 60 是风的密度。
    api.set_fluid(Wind {
        dx: 0,
        dy: 10,
        pressure: 0,
        density: 60,
    });

    // 2. 随机选择一个相邻格子并检查其物质类型
    // 这行代码使用 api.rand_vec() 生成一个随机的方向 (dx, dy)，用来选择一个相邻格子进行操作。
    let (dx, dy) = api.rand_vec();

    // 3. 与气体或灰尘交互
    // 如果随机选择的格子是气体（Species::Gas）或灰尘（Species::Dust），则在该格子中生成一个火（Species::Fire）。
    // 火的“活跃度”（ra）由 (150 + (dx + dy) * 10) 计算得到，rb 和 clock 则为 0。
    if api.get(dx, dy).species == Species::Gas || api.get(dx, dy).species == Species::Dust {
        api.set(
            dx,
            dy,
            Cell {
                species: Species::Fire,
                ra: (150 + (dx + dy) * 10) as u8,
                rb: 0,
                clock: 0,
            },
        );
    }

    // 4. 与水的交互
    //  如果随机选择的格子是水（Species::Water），则岩浆与水发生反应，岩浆变成石头（Species::Stone），并将石头放置在当前格子。
    // 同时，水被移除，设置为 EMPTY_CELL。
    //
    let sample = api.get(dx, dy);
    if sample.species == Species::Water {
        api.set(
            0,
            0,
            Cell {
                species: Species::Stone,
                ra: (150 + (dx + dy) * 10) as u8,
                rb: 0,
                clock: 0,
            },
        );
        api.set(dx, dy, EMPTY_CELL);

    //     5. 岩浆的移动
    // 接下来，岩浆尝试向周围的空白格子（Species::Empty）移动。如果周围的格子是空的，它会向该格子移动，否则保持当前位置。
    // 检查当前格子（0, 0）上下左右（0, 1、dx, 1、dx, 0）是否为空（Species::Empty）。
    // 如果某个方向的格子为空，则岩浆会向该方向移动。
    // 如果没有空格子可以移动，则岩浆保持在原位置。
    } else if api.get(0, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(0, 1, cell);
    } else if api.get(dx, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(dx, 1, cell);
    } else if api.get(dx, 0).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(dx, 0, cell);
    } else {
        api.set(0, 0, cell);
    }
}

// 模拟木材在沙盒模拟环境中的行为。木材的行为包括与火、熔岩、水等物质的互动，以及根据条件改变状态（如变为火或变为空）。
pub fn update_wood(cell: Cell, mut api: SandApi) {
    //
    let rb = cell.rb;

    let (dx, dy) = api.rand_vec();

    let nbr_species = api.get(dx, dy).species;

    // 1. 初始化与火或熔岩交互
    // 这段代码首先判断木材（Wood）是否处于初始状态（rb == 0）。如果是并且它周围的格子是火（Fire）或熔岩（Lava），则木材将变为状态 Wood，并且将其 rb 设置为 90。
    // ra 和 clock 由原始木材的属性继承
    if rb == 0 && nbr_species == Species::Fire || nbr_species == Species::Lava {
        api.set(
            0,
            0,
            Cell {
                species: Species::Wood,
                ra: cell.ra,
                rb: 90,
                clock: 0,
            },
        );
    }

    // 2. 如果木材的 rb 大于 1，则进行如下操作
    // 如果木材的 rb 大于 1，木材的 rb 减 1，并保持木材的其他属性（species 和 ra）不变。
    if rb > 1 {
        api.set(
            0,
            0,
            Cell {
                species: Species::Wood,
                ra: cell.ra,
                rb: rb - 1,
                clock: 0,
            },
        );
        // 3. 木材和空白格子、火的互动
        // 如果木材的 rb 是 4 的倍数并且相邻的格子为空（Species::Empty），则在该空格上生成一个火（Species::Fire）。火的 ra 是一个随机值，范围在 30 到 90 之间。
        if rb % 4 == 0 && nbr_species == Species::Empty {
            let ra = 30 + api.rand_int(60) as u8;
            api.set(
                dx,
                dy,
                Cell {
                    species: Species::Fire,
                    ra,
                    rb: 0,
                    clock: 0,
                },
            )
        }
        // 4. 木材和水的互动
        // 如果木材接触到水（Species::Water），则木材变成状态 Wood，并且 ra 设置为 50，rb 设置为 0。这种情况下，水可能会导致木材的某些改变，或者用风的模拟来表示水蒸气的作用。
        // api.set_fluid() 设置了一个流体属性，虽然具体作用没有完全显示，但可能与风的物理行为（例如水蒸气或湿度）有关。
        if nbr_species == Species::Water {
            api.set(
                0,
                0,
                Cell {
                    species: Species::Wood,
                    ra: 50,
                    rb: 0,
                    clock: 0,
                },
            );
            api.set_fluid(Wind {
                dx: 0,
                dy: 0,
                pressure: 0,
                density: 220,
            });
        }

    //     5. 如果 rb 为 1，则变为空格
    //     如果木材的 rb 为 1，则将其变为空格，并且 ra 设为原来木材的 ra，rb 设为 90，恢复为初始状态。
    } else if rb == 1 {
        api.set(
            0,
            0,
            Cell {
                species: Species::Empty,
                ra: cell.ra,
                rb: 90,
                clock: 0,
            },
        );
    }
}
// 你的代码用于模拟冰（Ice）的行为，主要涉及冰与火、岩浆、水等物质的互动。代码的逻辑比较清晰
pub fn update_ice(cell: Cell, mut api: SandApi) {
    let (dx, dy) = api.rand_vec();

    let i = api.rand_int(100);

    let fluid = api.get_fluid();

    // // 如果流体压力大于120且有一定概率，冰会变成水
    if fluid.pressure > 120 && api.rand_int(1) == 0 {
        api.set(
            0,
            0,
            Cell {
                species: Species::Water,
                ra: cell.ra,
                rb: 0,
                clock: 0,
            },
        );
        return;
    }

    let nbr_species = api.get(dx, dy).species;
    // // 如果邻居是火或岩浆，冰会变成水
    if nbr_species == Species::Fire || nbr_species == Species::Lava {
        api.set(
            0,
            0,
            Cell {
                species: Species::Water,
                ra: cell.ra,
                rb: cell.rb,
                clock: 0,
            },
        );
    } else if nbr_species == Species::Water && i < 7 {
        // 如果邻居是水且随机条件成立，冰会变成冰块
        api.set(
            dx,
            dy,
            Cell {
                species: Species::Ice,
                ra: cell.ra,
                rb: cell.rb,
                clock: 0,
            },
        );
    }
}


// 涉及两种不同物质（植物 Plant 和种子 Seed）的行为逻辑

// 该函数描述了植物（Plant）的生长和互动行为，主要操作包括扩散、繁殖、腐蚀、与其他物质互动等。
pub fn update_plant(cell: Cell, mut api: SandApi) {

    // 植物与火焰或岩浆的交互：
    // 如果植物周围有火焰（Fire）或岩浆（Lava），并且植物的rb为0，它会变成一个新的植物，ra保持不变，rb设置为20（表示一些生长状态）。
    let rb = cell.rb;

    let mut i = api.rand_int(100);
    let (dx, dy) = api.rand_vec();

    let nbr_species = api.get(dx, dy).species;
    if rb == 0 && nbr_species == Species::Fire || nbr_species == Species::Lava {
        api.set(
            0,
            0,
            Cell {
                species: Species::Plant,
                ra: cell.ra,
                rb: 20,
                clock: 0,
            },
        );
    }

    // 2 与木材的交互：
    // 如果植物旁边有木材（Wood），它会随机选择一个邻居位置（dx, dy），并将植物繁殖到这个空白位置。
    if nbr_species == Species::Wood {
        let (dx, dy) = api.rand_vec();

        let drift = (i % 15) - 7;
        let newra = (cell.ra as i32 + drift) as u8;
        if api.get(dx, dy).species == Species::Empty {
            api.set(
                dx,
                dy,
                Cell {
                    species: Species::Plant,
                    ra: newra,
                    rb: 0,
                    clock: 0,
                },
            );
        }
    }
    // 3 与水或真菌的交互：
    //
    // 如果植物旁边有水（Water）或真菌（Fungus），并且与相邻位置进行某种条件匹配，植物会随机选择一个邻居并繁殖到该位置，同时移除另一个方向上的植物。
    if api.rand_int(100) > 80
        && (nbr_species == Species::Water
        || nbr_species == Species::Fungus
        && (api.get(-dx, dy).species == Species::Empty
        || api.get(-dx, dy).species == Species::Water
        || api.get(-dx, dy).species == Species::Fungus))
    {
        i = api.rand_int(100);
        let drift = (i % 15) - 7;
        let newra = (cell.ra as i32 + drift) as u8;
        api.set(
            dx,
            dy,
            Cell {
                ra: newra,
                rb: 0,
                ..cell
            },
        );
        api.set(-dx, dy, EMPTY_CELL);
    }
    // 4 植物的生命周期：
    //
    // 如果植物的rb大于1，它会减少rb并产生火焰（Fire）或者将植物的ra调整为50，如果附近是水。
    // 如果rb为1，植物会被清除（设置为空）。
    if rb > 1 {
        api.set(
            0,
            0,
            Cell {
                ra: cell.ra,
                rb: rb - 1,
                ..cell
            },
        );

        if nbr_species == Species::Empty {
            let ra = 20 + api.rand_int(30) as u8;
            api.set(
                dx,
                dy,
                Cell {
                    species: Species::Fire,
                    ra,
                    rb: 0,
                    clock: 0,
                },
            );
        }
        if nbr_species == Species::Water {
            api.set(
                0,
                0,
                Cell {
                    ra: 50,
                    rb: 0,
                    ..cell
                },
            )
        }
    } else if rb == 1 {
        api.set(0, 0, EMPTY_CELL);
    }

    //  5 植物繁殖：
    //
    // 如果植物的ra大于50，并且在某个条件下没有相邻的植物，植物会在上方繁殖。
    let ra = cell.ra;
    if ra > 50
        && api.get(1, 1).species != Species::Plant
        && api.get(-1, 1).species != Species::Plant
    {
        if api.get(0, 1).species == Species::Empty {
            let i = (rand::random::<f32>() * rand::random::<f32>() * 100.) as i32;
            let dec = api.rand_int(30) - 20;
            if (i + ra as i32) > 165 {
                api.set(
                    0,
                    1,
                    Cell {
                        ra: (ra as i32 + dec) as u8,
                        ..cell
                    },
                );
            }
        } else {
            api.set(
                0,
                0,
                Cell {
                    ra: (ra - 1) as u8,
                    ..cell
                },
            );
        }
    }
}

// 描述了种子（Seed）的行为逻辑。它实现了种子从空中掉落、与周围物质的互动以及生长和扩展等行为。
pub fn update_seed(cell: Cell, mut api: SandApi) {
    // 1 火焰与岩浆的处理：
    //
    // 如果种子附近有火焰（Fire）或岩浆（Lava），种子会变为火焰（Fire），并且其属性被设置为 ra: 5，rb: 0。
    let rb = cell.rb;
    let ra = cell.ra;

    let (dx, dy) = api.rand_vec();

    let nbr_species = api.get(dx, dy).species;
    if nbr_species == Species::Fire || nbr_species == Species::Lava {
        api.set(
            0,
            0,
            Cell {
                species: Species::Fire,
                ra: 5,
                rb: 0,
                clock: 0,
            },
        );
        return;
    }

    // 2 种子掉落逻辑：
    //
    // 如果种子正在掉落（rb == 0），它会根据周围的环境进行调整：
    // 如果种子下面是沙子（Sand）、植物（Plant）或真菌（Fungus），它会停止掉落并生成一个新的生命值（rb）。
    // 如果种子落在空白位置，种子会继续掉落到下方。
    // 如果种子下方是水（Water）、气体（Gas）、油（Oil）或酸（Acid），种子会与之互动并继续下落。
    if rb == 0 {
        //falling

        let dxf = api.rand_dir(); //falling dx
        let nbr_species_below = api.get(dxf, 1).species;
        if nbr_species_below == Species::Sand
            || nbr_species_below == Species::Plant
            || nbr_species_below == Species::Fungus
        {
            let rb = (api.rand_int(253) + 1) as u8;
            api.set(0, 0, Cell { rb, ..cell });
            return;
        }

        let nbr = api.get(0, 1);
        if nbr.species == Species::Empty {
            api.set(0, 0, EMPTY_CELL);
            api.set(0, 1, cell);
        } else if api.get(dxf, 1).species == Species::Empty {
            api.set(0, 0, EMPTY_CELL);
            api.set(dxf, 1, cell);
        } else if nbr.species == Species::Water
            || nbr.species == Species::Gas
            || nbr.species == Species::Oil
            || nbr.species == Species::Acid
        {
            api.set(0, 0, nbr);
            api.set(0, 1, cell);
        } else {
            api.set(0, 0, cell);
        }
    } else {

        // 3 种子生长为茎（stem）：
        //
        // 如果种子的腐蚀度（ra）大于 60，种子有可能向上生成茎（stem）。
        // 如果上方的某个位置为空白、沙子或者是其他种子，且两侧位置没有植物，种子会生成茎并逐步变成植物。
        if ra > 60 {
            //stem
            let dxr = api.rand_dir(); //raising dx
            if api.rand_int(100) > 75 {
                if (api.get(dxr, -1).species == Species::Empty
                    || api.get(dxr, -1).species == Species::Sand
                    || api.get(dxr, -1).species == Species::Seed)
                    && api.get(1, -1).species != Species::Plant
                    && api.get(-1, -1).species != Species::Plant
                {
                    let ra = (ra as i32 - api.rand_int(10)) as u8;
                    api.set(dxr, -1, Cell { ra, ..cell });
                    let ra2 = 80 + api.rand_int(30) as u8;
                    api.set(
                        0,
                        0,
                        Cell {
                            species: Species::Plant,
                            ra: ra2,
                            rb: 0,
                            clock: 0,
                        },
                    )
                } else {
                    api.set(0, 0, EMPTY_CELL);
                }
            }
        } else {
            // 4 种子生成花瓣（petals）：
            //
            // 如果腐蚀度（ra）小于 60，但大于 40，种子有可能在邻近位置生成花瓣。
            // 这个过程通过对周围空白区域或植物位置的检查来决定是否生成新的花瓣。
            if ra > 40 {
                //petals

                let (mdx, mdy) = api.rand_vec();

                let (ldx, ldy) = adjacency_left((mdx, mdy));
                let (rdx, rdy) = adjacency_right((mdx, mdy));

                if (api.get(mdx, mdy).species == Species::Empty
                    || api.get(mdx, mdy).species == Species::Plant)
                    && (api.get(ldx, ldy).species == Species::Empty
                    || api.get(rdx, rdy).species == Species::Empty)
                {
                    let i = (rand::random::<f32>() * rand::random::<f32>() * 100.) as i32;
                    let dec = 9 - api.rand_int(3);
                    if (i + ra as i32) > 100 {
                        api.set(
                            mdx,
                            mdy,
                            Cell {
                                ra: (ra as i32 - dec) as u8,
                                ..cell
                            },
                        );
                    }
                }
            } else {

                // 5 与水的互动：
                //
                // 如果种子附近是水（Water），种子会转变为新的一颗种子（Species::Seed）。
                if nbr_species == Species::Water {
                    api.set(dx, dy, Cell::new(Species::Seed))
                }
            }
        }
    }
}

// 这段代码实现了一个类似“真菌”物质的行为逻辑，真菌在周围的环境中扩散、生长、腐蚀或转化。
pub fn update_fungus(cell: Cell, mut api: SandApi) {
    // 1 初始化：
    //
    // let rb = cell.rb;：读取当前真菌细胞的生命值 rb。
    // let (dx, dy) = api.rand_vec();：生成一个随机的方向向量（dx 和 dy），用于决定真菌的扩散方向。
    // let nbr_species = api.get(dx, dy).species;：获取相邻单元格的物质种类。
    let rb = cell.rb;

    let (dx, dy) = api.rand_vec();

    let nbr_species = api.get(dx, dy).species;

    // 2 火焰与岩浆扩散：
    //
    // 如果当前 rb == 0 且相邻的单元格是火焰（Fire）或岩浆（Lava），则创建一个新的真菌并将其放置在当前位置。
    // api.set(0, 0, Cell {...})：将当前位置的细胞替换为新的真菌细胞。
    if rb == 0 && nbr_species == Species::Fire || nbr_species == Species::Lava {
        api.set(
            0,
            0,
            Cell {
                species: Species::Fungus,
                ra: cell.ra,
                rb: 10,
                clock: 0,
            },
        );
    }

    // 3 随机扩散：
    //
    // let mut i = api.rand_int(100);：生成一个随机值，用于确定真菌是否扩散。

    let mut i = api.rand_int(100);
    //  // 如果相邻单元格不是空的 (Species::Empty)，也不是真菌 (Species::Fungus)、
    // 火焰 (Species::Fire) 或冰 (Species::Ice)，则真菌尝试扩散到一个空的邻近单元格。
    //
    if nbr_species != Species::Empty
        && nbr_species != Species::Fungus
        && nbr_species != Species::Fire
        && nbr_species != Species::Ice
    {

        let (dx, dy) = api.rand_vec();

        let drift = (i % 15) - 7;

        //  // 真菌的腐蚀程度 ra 会增加或减少，生成新的 ra 值并将其放置到新的位置。
        let newra = (cell.ra as i32 + drift) as u8;
        if api.get(dx, dy).species == Species::Empty {
            api.set(
                dx,
                dy,
                Cell {
                    species: Species::Fungus,
                    ra: newra,
                    rb: 0,
                    clock: 0,
                },
            );
        }
    }

    // 4 真菌与木材的相互作用：
    //
    // 如果相邻单元格是木材（Species::Wood），且满足一定条件（例如相邻木材不在真菌的影响范围内），
    // 真菌会扩散到相邻的木材单元格。
    if i > 9
        && nbr_species == Species::Wood
        && api.get(-dx, dy).species == Species::Wood
        && api.get(dx, -dy).species == Species::Wood
        && api.get(dx, dy).ra % 4 != 0
    {
        i = api.rand_int(100);
        let drift = (i % 15) - 7;
        let newra = (cell.ra as i32 + drift) as u8;
        api.set(
            dx,
            dy,
            Cell {
                ra: newra,
                rb: 0,
                ..cell
            },
        );
    }
    // 5 生命值 (rb) 管理：
    //
    // 如果 rb > 1，真菌会减少其生命值（rb）并检查周围是否为空或水等物质。如果为空，生成火种（Species::Fire）；如果是水，则改变真菌的状态。
    // 如果 rb == 1，真菌将消失（设置为 EMPTY_CELL）。
    if rb > 1 {
        api.set(
            0,
            0,
            Cell {
                ra: cell.ra,
                rb: rb - 1,
                ..cell
            },
        );
        if nbr_species == Species::Empty {
            let ra = 10 + api.rand_int(10) as u8;
            api.set(
                dx,
                dy,
                Cell {
                    species: Species::Fire,
                    ra,
                    rb: 0,
                    clock: 0,
                },
            )
        }
        if nbr_species == Species::Water {
            api.set(
                0,
                0,
                Cell {
                    ra: 50,
                    rb: 0,
                    ..cell
                },
            )
        }
    //     如果 rb == 1，真菌将消失（设置为 EMPTY_CELL）。
    } else if rb == 1 {
        api.set(0, 0, EMPTY_CELL);
    }

    let ra = cell.ra;

    // 6  高级扩散：
    //
    // 如果真菌的腐蚀程度 ra 大于 120，真菌会尝试在一个新的随机位置扩散，并且该位置必须满足一系列条件（例如不与其他真菌接触）。
    // 如果满足条件，真菌会扩散并且腐蚀程度会减少。
    if ra > 120 {
        let (mdx, mdy) = api.rand_vec();

        let (ldx, ldy) = adjacency_left((mdx, mdy));
        let (rdx, rdy) = adjacency_right((mdx, mdy));
        if api.get(mdx, mdy).species == Species::Empty
            && api.get(ldx, ldy).species != Species::Fungus
            && api.get(rdx, rdy).species != Species::Fungus
        {
            let i = (rand::random::<f32>() * rand::random::<f32>() * 100.) as i32;
            let dec = 15 - api.rand_int(20);
            if (i + ra as i32) > 165 {
                api.set(
                    mdx,
                    mdy,
                    Cell {
                        ra: (ra as i32 - dec) as u8,
                        ..cell
                    },
                );
            }
        }
    }
}

// 这段代码实现了酸（Acid）物质的行为逻辑，根据不同的条件酸会向周围扩散、腐蚀或退化。代码通过检查周围单元格的物质种类来决定酸的移动或变更。
pub fn update_acid(cell: Cell, mut api: SandApi) {
    // 1.方向控制：
    //
    // let dx = api.rand_dir(); 随机决定一个方向，dx 代表水平方向的移动量（可以是 1 或 -1）
    let dx = api.rand_dir();

    //2. 酸的退化：
    //
    // let ra = cell.ra; 获取当前酸的腐蚀程度。
    // let mut degraded = cell.clone(); 创建酸的副本。
    // degraded.ra = ra - 60; 酸的腐蚀程度减少 60，表示酸的退化。
    // 如果酸的腐蚀程度小于 80（degraded.ra < 80），则酸会消失（设置为空单元格 EMPTY_CELL）。
    let ra = cell.ra;
    let mut degraded = cell.clone();
    degraded.ra = ra - 60;
    // i = api.rand_int(100);
    if degraded.ra < 80 {
        degraded = EMPTY_CELL;
    }

    // 3.酸的扩散：
    //
    // 通过检查四个方向（上、右、左、下）的相邻单元格，酸决定是否扩散到这些空白区域或腐蚀周围的物质。
    // 优先向下移动（api.get(0, 1)），如果下方为空，则酸向下扩散。
    // 如果下方不是空单元格，尝试向右（api.get(dx, 0)）或向左（api.get(-dx, 0)）移动。
    // 如果四个方向都被阻挡（例如遇到墙壁 Species::Wall 或酸 Species::Acid），酸会检查是否能向上（api.get(0, -1)) 移动。
    // 向下
    if api.get(0, 1).species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(0, 1, cell);
    } else if api.get(dx, 0).species == Species::Empty {
        // 向右
        api.set(0, 0, EMPTY_CELL);
        api.set(dx, 0, cell);
    } else if api.get(-dx, 0).species == Species::Empty {
        // 向左
        api.set(0, 0, EMPTY_CELL);
        api.set(-dx, 0, cell);
    } else {
        // 向上
        if api.get(0, 1).species != Species::Wall && api.get(0, 1).species != Species::Acid {
            api.set(0, 0, EMPTY_CELL);
            api.set(0, 1, degraded);
        } else if api.get(dx, 0).species != Species::Wall && api.get(dx, 0).species != Species::Acid
        {
            api.set(0, 0, EMPTY_CELL);
            api.set(dx, 0, degraded);
        } else if api.get(-dx, 0).species != Species::Wall
            && api.get(-dx, 0).species != Species::Acid
        {
            api.set(0, 0, EMPTY_CELL);
            api.set(-dx, 0, degraded);
        } else if api.get(0, -1).species != Species::Wall
            && api.get(0, -1).species != Species::Acid
            && api.get(0, -1).species != Species::Empty
        {
            // 4 酸的腐蚀行为：
            //
            // 如果酸能够移动到空单元格，它会将自己放到新位置，并将当前单元格清空。
            // 如果周围不是空单元格，酸会腐蚀（退化）周围的物质。如果周围的物质是墙壁或酸，酸不会继续腐蚀。否则，它会把腐蚀后的酸放到该位置。
            api.set(0, 0, EMPTY_CELL);
            api.set(0, -1, degraded);
        } else {
            // 5 回退行为：
            //
            // 如果酸没有找到可以移动或腐蚀的地方，它会保持在当前位置。
            api.set(0, 0, cell);
        }
    }
}

pub fn update_mite(cell: Cell, mut api: SandApi) {
    // 1：初始设置：
    // 生成一个随机整数，dx 和 dy 代表螨虫的移动方向。
    // 根据 cell.ra 和 cell.rb 的值来调整 dx 和 dy，这决定了螨虫的移动方向。
    let mut i = api.rand_int(100);
    let mut dx = 0;

    //2 方向控制：
    //
    // 如果 cell.ra < 20，螨虫向左移动（dx = (cell.ra as i32) - 1）。
    // 如果 cell.rb > 10，螨虫向上移动（dy = -1），否则向下移动（dy = 1）。
    if cell.ra < 20 {
        dx = (cell.ra as i32) - 1;
    }
    let mut dy = 1;
    let mut mite = cell.clone();


    if cell.rb > 10 {
        // /
        mite.rb = mite.rb.saturating_sub(1);
        dy = -1;
    } else if cell.rb > 1 {
        // \
        mite.rb = mite.rb.saturating_sub(1);
    } else {
        // |
        dx = 0;
    }
    //3 与邻居的互动：
    //
    // nbr = api.get(dx, dy)：获取螨虫将要移动到的目标位置的单元格。
    // 随机决定 sx 和 sy，来采样周围的单元格。
    let nbr = api.get(dx, dy);

    let sx = (i % 3) - 1;
    i = api.rand_int(1000);
    let sy = (i % 3) - 1;
    let sample = api.get(sx, sy).species;

    // 4与特定物质的互动：
    //
    // 如果采样的物质是火（Fire）、岩浆（Lava）、水（Water）或油（Oil），螨虫会消失（设置为空单元格 EMPTY_CELL）。

    if sample == Species::Fire
        || sample == Species::Lava
        || sample == Species::Water
        || sample == Species::Oil
    {
        api.set(0, 0, EMPTY_CELL);
        return;
    }
    // // 如果采样的是植物类物质（Plant、Wood、Seed），并且随机值 i > 800，螨虫会移动到该位置。
    if (sample == Species::Plant || sample == Species::Wood || sample == Species::Seed) && i > 800 {
        api.set(0, 0, EMPTY_CELL);
        api.set(sx, sy, cell);

        return;
    }

    // 5处理尘土：
    //
    // 如果采样的物质是尘土（Dust），螨虫会根据随机概率决定是否消失或者停留
    if sample == Species::Dust {
        api.set(sx, sy, if i > 800 { cell } else { EMPTY_CELL });
    }
    // 6 螨虫的移动：
    //
    // 如果目标位置是空的（Species::Empty），螨虫会移动到那里。
    if nbr.species == Species::Empty {
        api.set(0, 0, EMPTY_CELL);
        api.set(dx, dy, mite);
    } else if dy == 1 && i > 800 {
        // 如果周围被其它螨虫阻塞，螨虫可能会卡住或者改变方向。

        i = api.rand_int(100);
        let mut ndx = (i % 3) - 1;
        if i < 6 {
            //switch direction
            ndx = dx;
        }

        mite.ra = (1 + ndx) as u8;
        mite.rb = 10 + (i % 10) as u8; //hop height

        api.set(0, 0, mite);
    } else {
        // 如果周围是冰块，螨虫会尝试在冰面上移动或者爬升。
        if api.get(-1, 0).species == Species::Mite
            && api.get(1, 0).species == Species::Mite
            && api.get(0, -1).species == Species::Mite
        {
            api.set(0, 0, EMPTY_CELL);
        } else {
            //7 最终状态更新：
            //
            // 更新螨虫的状态，或者将其移除并将空单元格设置在当前位置。
            if api.get(0, 1).species == Species::Ice {
                if api.get(dx, 0).species == Species::Empty {
                    api.set(0, 0, EMPTY_CELL);
                    api.set(dx, 0, mite);
                }
            } else {
                api.set(0, 0, mite);
            }
        }
    }
}



// 如果输入是 (0, 1)（向上），右侧方向就是 (1, 1)（向右上）。
// 如果输入是 (1, 1)（向右上），右侧方向是 (1, 0)（向右）。
// 以此类推，旋转 90 度得到顺时针的右侧邻近方向。
pub fn adjacency_right(dir: (i32, i32)) -> (i32, i32) {
    match dir {
        (0, 1) => (1, 1),
        (1, 1) => (1, 0),
        (1, 0) => (1, -1),
        (1, -1) => (0, -1),
        (0, -1) => (-1, -1),
        (-1, -1) => (-1, 0),
        (-1, 0) => (-1, 1),
        (-1, 1) => (0, 1),
        _ => (0, 0),
    }
}

// 这段代码与 adjacency_right 相似，但它返回的是逆时针方向的邻近位置。
// 比如，输入 (0, 1)（向上），左侧方向是 (-1, 1)（向左上）。
pub fn adjacency_left(dir: (i32, i32)) -> (i32, i32) {
    match dir {
        (0, 1) => (-1, 1),
        (1, 1) => (0, 1),
        (1, 0) => (1, 1),
        (1, -1) => (1, 0),
        (0, -1) => (1, -1),
        (-1, -1) => (0, -1),
        (-1, 0) => (-1, -1),
        (-1, 1) => (-1, 0),
        _ => (0, 0),
    }
}


// join_dy_dx 和 split_dy_dx
// 这两个函数负责在不同的数值表示和方向之间进行转换，主要用于将两个方向的值合并成一个 u8 或从一个 u8 恢复成两个方向值。

// 该函数将 dx 和 dy 两个方向值（取值范围是 -1、0、1）合并成一个 u8 值。通过这种方式，可以方便地存储和传递方向。
// 例如，如果 dx = 1 和 dy = 0，那么 join_dy_dx(1, 0) 会计算为 ((1 + 1) * 3 + (0 + 1)) = 7，返回 7。
pub fn join_dy_dx(dx: i32, dy: i32) -> u8 {
    (((dx + 1) * 3) + (dy + 1)) as u8
}


// 该函数将 u8 类型的方向值解码成 dx 和 dy 两个整数，恢复方向。
// 例如，split_dy_dx(7) 会将 7 分解回 (1, 0)。
pub fn split_dy_dx(s: u8) -> (i32, i32) {
    let s: i32 = s as i32;

    let dx: i32 = (s / 3) - 1;

    let dy: i32 = (s % 3) - 1;

    (dx, dy)
}

use js_sys;
