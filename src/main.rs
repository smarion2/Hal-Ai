extern crate rand;

use hlt::command::Command;
use hlt::direction::Direction;
use hlt::game::Game;
use rand::Rng;
use rand::SeedableRng;
use rand::XorShiftRng;
use std::env;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::collections::HashMap;

mod hlt;

fn main() {
    let args: Vec<String> = env::args().collect();
    let rng_seed: u64 = if args.len() > 1 {
        args[1].parse().unwrap()
    } else {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    };
    let seed_bytes: Vec<u8> = (0..16).map(|x| ((rng_seed >> (x % 8)) & 0xFF) as u8).collect();
    let mut rng: XorShiftRng = SeedableRng::from_seed([
        seed_bytes[0], seed_bytes[1], seed_bytes[2], seed_bytes[3],
        seed_bytes[4], seed_bytes[5], seed_bytes[6], seed_bytes[7],
        seed_bytes[8], seed_bytes[9], seed_bytes[10], seed_bytes[11],
        seed_bytes[12], seed_bytes[13], seed_bytes[14], seed_bytes[15]
    ]);

    let mut game = Game::new();
    // At this point "game" variable is populated with initial map data.
    // This is a good place to do computationally expensive start-up pre-processing.
    // As soon as you call "ready" function below, the 2 second per turn timer will start.
    Game::ready("smarion2");
    let mut ship_status = HashMap::new();
    game.log.borrow_mut().log(&format!("Successfully created bot! My Player ID is {}. Bot rng seed is {}.", game.my_id.0, rng_seed));

    loop {
        game.update_frame();
        let me = &game.players[game.my_id.0];

        let mut command_queue: Vec<Command> = Vec::new();
        let shipyard = &me.shipyard;
        game.log.borrow_mut().log(&format!("dropoff length {}.", shipyard.position.x));
        for ship_id in &me.ship_ids {
            let ship = &game.ships[ship_id];
            let id = ship_id.0;
            let halite = game.game_map.at_entity(ship).halite;
            if !ship_status.contains_key(&id) {
                ship_status.insert(id, "exploring".to_string());
            }
            else if ship_status[&id] == "returning" {
                if &ship.position != &shipyard.position {
                    let towards_dropoff = &game.game_map.naive_navigate(ship, &shipyard.position);
                    let command = ship.move_ship(*towards_dropoff);
                    command_queue.push(command);
                    continue;
                } else {
                    ship_status.insert(id, "exploring".to_string());
                };
            }
            else if ship.halite >= game.constants.max_halite - 250 || game.turns_left() <= game.game_map.height {
                ship_status.insert(id, "returning".to_string());
            }

            let command = if halite < 50 || ship.is_full() {
                let random_direction = Direction::get_all_cardinals()[rng.gen_range(0, 4)];                
                let safe_pos = &game.game_map.naive_navigate(ship, &ship.position.directional_offset(random_direction));
                ship.move_ship(*safe_pos)                
            } else if game.turns_left() <= game.game_map.height {
                let mut closest_drop = game.game_map.calculate_distance(&ship.position, &shipyard.position);
                let mut closest_pos = shipyard.position;
                for dropoff_id in &me.dropoff_ids {
                    let dropoff = &game.dropoffs[&dropoff_id];
                    let dropoff_distance = game.game_map.calculate_distance(&ship.position, &dropoff.position);
                    if closest_drop > dropoff_distance {
                        closest_drop = dropoff_distance;
                        closest_pos = dropoff.position;
                    }
                }
                let towards_closest = &game.game_map.naive_navigate(&ship, &closest_pos);
                ship.move_ship(*towards_closest)
            } else {
                ship.stay_still()
            };
            command_queue.push(command);
        }

        let shipyard_cell = game.game_map.at_entity(&me.shipyard);

        if
            game.turn_number <= 200 &&
            me.halite >= game.constants.ship_cost &&
            !shipyard_cell.is_occupied()
        {
            command_queue.push(me.shipyard.spawn());
        }


        Game::end_turn(&command_queue);
    }
}
