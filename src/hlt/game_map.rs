use hlt::direction::Direction;
use hlt::entity::Entity;
use hlt::input::Input;
use hlt::map_cell::MapCell;
use hlt::map_cell::Structure;
use hlt::position::Position;
use hlt::ship::Ship;
use std::cmp::min;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

pub struct GameMap {
    pub width: usize,
    pub height: usize,
    cells: Vec<Vec<MapCell>>,
}

impl GameMap {
    pub fn at_position(&self, position: &Position) -> &MapCell {
        let normalized = self.normalize(position);
        &self.cells[normalized.y as usize][normalized.x as usize]
    }

    pub fn at_position_mut(&mut self, position: &Position) -> &mut MapCell {
        let normalized = self.normalize(position);
        &mut self.cells[normalized.y as usize][normalized.x as usize]
    }

    pub fn at_entity(&self, entity: &Entity) -> &MapCell {
        self.at_position(&entity.position())
    }

    pub fn at_entity_mut(&mut self, entity: &Entity) -> &mut MapCell {
        self.at_position_mut(&entity.position())
    }

    pub fn calculate_distance(&self, source: &Position, target: &Position) -> usize {
        let normalized_source = self.normalize(source);
        let normalized_target = self.normalize(target);

        let dx = (normalized_source.x - normalized_target.x).abs() as usize;
        let dy = (normalized_source.y - normalized_target.y).abs() as usize;

        let toroidal_dx = min(dx, self.width - dx);
        let toroidal_dy = min(dy, self.height - dy);

        return toroidal_dx + toroidal_dy;
    }

    pub fn normalize(&self, position: &Position) -> Position {
        let width = self.width as i32;
        let height = self.height as i32;
        let x = ((position.x % width) + width) % width;
        let y = ((position.y % height) + height) % height;
        Position { x, y }
    }

    pub fn get_unsafe_moves(&self, source: &Position, destination: &Position) -> Vec<Direction> {
        let normalized_source = self.normalize(source);
        let normalized_destination = self.normalize(destination);

        let dx = (normalized_source.x - normalized_destination.x).abs() as usize;
        let dy = (normalized_source.y - normalized_destination.y).abs() as usize;

        let wrapped_dx = self.width - dx;
        let wrapped_dy = self.height - dy;

        let mut possible_moves: Vec<Direction> = Vec::new();

        if normalized_source.x < normalized_destination.x {
            possible_moves.push(if dx > wrapped_dx { Direction::West } else { Direction::East });
        } else if normalized_source.x > normalized_destination.x {
            possible_moves.push(if dx < wrapped_dx { Direction::West } else { Direction::East });
        }

        if normalized_source.y < normalized_destination.y {
            possible_moves.push(if dy > wrapped_dy { Direction::North } else { Direction::South });
        } else if normalized_source.y > normalized_destination.y {
            possible_moves.push(if dy < wrapped_dy { Direction::North } else { Direction::South });
        }

        possible_moves
    }

    pub fn naive_navigate(&mut self, ship: &Ship, destination: &Position) -> Direction {
        let ship_position = &ship.position;

        // get_unsafe_moves normalizes for us
        for direction in self.get_unsafe_moves(&ship_position, destination) {
            let target_pos = ship_position.directional_offset(direction);
            let target_cell = self.at_position_mut(&target_pos);

            if !target_cell.is_occupied() {
                target_cell.mark_unsafe(ship.id);
                return direction;
            }
        }

        Direction::Still
    }

    pub fn most_halite_near_ship_direction(&mut self, position: &Position) -> Option<Direction> {     
        let mut most_halite = 0;
        let mut best_direction = Direction::Still;
        let current_pos = position;

        for direction in Direction::get_all_cardinals() {
            let target_pos = current_pos.directional_offset(direction);
            let cell = self.at_position(&target_pos);
            if !cell.is_occupied() && cell.halite > most_halite {
                most_halite = cell.halite;
                best_direction = direction;
            }
        }        
        if most_halite > 10 {
            Some(best_direction)
        } else {
            None
        }
    }

    pub fn move_towards_rich_halite(&mut self, position: &Position) -> Direction {
        let mut best_direction = Direction::Still;
        let mut lowest_distance = 0;
        for direction in Direction::get_all_cardinals() {
            let mut distance = 0;
            let mut current_pos = *position;
            let mut move_not_found = false;
            while self.at_position(&current_pos).halite < 25 {
                distance += 1;
                current_pos = current_pos.directional_offset(direction);
                let cell = self.at_position(&current_pos);
                if (cell.is_occupied() && distance == 1) || distance > 10 {
                    move_not_found = true;
                    break;
                }
            }
            if (lowest_distance == 0 || distance < lowest_distance) && !move_not_found {
                lowest_distance = distance;
                best_direction = direction;
            }
        }
        best_direction
    }

    pub fn find_suitable_dropoffs(&mut self) -> Vec<Position> {
        let mut possible_dropoffs: Vec<Position> = Vec::new();
        let mut heap = BinaryHeap::new();
        let (num_of_dropoffs, zone_radius) = if self.width < 33 {
            (2, 3i32)
        } else if self.width < 50 {
            (3, 4i32) 
        } else if self.width < 70 {
            (4, 5i32)
        } else {
            (5, 6i32)
        };
        for x in 0..self.width {
            for y in 0..self.height {
                let mut total_halite = 0;
                for radx in -zone_radius..zone_radius {
                    for rady in -zone_radius..zone_radius {
                        let posx = x as i32 + radx;
                        let posy = y as i32 + rady;
                        let pos = Position { x: posx, y: posy };
                        let norm_pos = self.normalize(&pos);
                        total_halite += self.at_position(&norm_pos).halite;
                    }
                }
                heap.push(HaliteScore { score: total_halite, x: x as i32, y: y as i32});
            }
        }
        let mut i = 0;
        while let Some(HaliteScore { score: _, x, y }) = heap.pop() {
            possible_dropoffs.push(Position { x: x, y: y});
            i += 1;
            if i == num_of_dropoffs {
                break;
            }
        }
        
        possible_dropoffs
    }

    pub fn update(&mut self, input: &mut Input) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.cells[y][x].ship = None;
            }
        }

        input.read_and_parse_line();
        let update_count = input.next_usize();

        for _ in 0..update_count {
            input.read_and_parse_line();
            let x = input.next_usize();
            let y = input.next_usize();
            let halite = input.next_usize();

            self.cells[y][x].halite = halite;
        }
    }

    pub fn generate(input: &mut Input) -> GameMap {
        input.read_and_parse_line();
        let width = input.next_usize();
        let height = input.next_usize();

        let mut cells: Vec<Vec<MapCell>> = Vec::with_capacity(height);
        for y in 0..height {
            input.read_and_parse_line();

            let mut row: Vec<MapCell> = Vec::with_capacity(width);
            for x in 0..width {
                let halite = input.next_usize();

                let position = Position { x: x as i32, y: y as i32 };
                let cell = MapCell { position, halite, ship: None, structure: Structure::None };
                row.push(cell);
            }

            cells.push(row);
        }

        GameMap { width, height, cells }
    }
}

#[derive(Eq, PartialEq)]
struct HaliteScore {
    score: usize,
    x: i32,
    y: i32
}

impl Ord for HaliteScore {
    fn cmp(&self, other: &HaliteScore) -> Ordering {
        self.score.cmp(&other.score)
            .then_with(|| self.x.cmp(&other.x))
    }
}

impl PartialOrd for HaliteScore {
    fn partial_cmp(&self, other: &HaliteScore) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}