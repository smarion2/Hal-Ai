extern crate rand;

use hlt::command::Command;
use hlt::game::Game;
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

    let mut game = Game::new();
    // At this point "game" variable is populated with initial map data.
    // This is a good place to do computationally expensive start-up pre-processing.
    // As soon as you call "ready" function below, the 2 second per turn timer will start.
    Game::ready("smarion2-new");
    let mut ship_status = HashMap::new();
    game.log.borrow_mut().log(&format!("Successfully created bot! My Player ID is {}. Bot rng seed is {}.", game.my_id.0, rng_seed));
    let best_dropoffs = game.game_map.find_suitable_dropoffs();
    for dropoff in &best_dropoffs {
        game.log.borrow_mut().log(&format!("Best drop off found x:{} y:{}.", dropoff.x, dropoff.y));    
    }
    let mut building_dropoff = false;
    loop {
        game.update_frame();
        let me = &game.players[game.my_id.0];

        let mut command_queue: Vec<Command> = Vec::new();
        let shipyard = &me.shipyard;  
        for ship_id in &me.ship_ids {
            let ship = &game.ships[ship_id];
            let id = ship_id.0;
            let halite = game.game_map.at_entity(ship).halite;
            if !ship_status.contains_key(&id) {
                ship_status.insert(id, "exploring".to_string());
            } else if game.turns_left() <= game.game_map.height - 15 {
                ship_status.insert(id, "rush_return".to_string());
            } 
            if ship_status[&id] == "returning" {
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
                if &ship.position != &closest_pos {
                    game.log.borrow_mut().log(&format!("returning ship {}.", id));
                    let towards_dropoff = &game.game_map.naive_navigate(ship, &closest_pos);
                    let command = ship.move_ship(*towards_dropoff);
                    command_queue.push(command);
                    continue;
                } else {
                    ship_status.insert(id, "exploring".to_string());
                };
            }
            else if ship_status[&id] == "rush_return" {
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
                let towards_closest = game.game_map.get_unsafe_moves(&ship.position, &closest_pos);                
                for moves in towards_closest {
                    let command = ship.move_ship(moves);
                    command_queue.push(command);
                    break;
                }
                continue;
            } else if ship_status[&id].contains("dropoff") {
                let drop_id = &ship_status[&id][7..];
                let drop_pos = best_dropoffs[drop_id.parse::<usize>().unwrap()];
                game.log.borrow_mut().log(&format!("ship turning into dropoff: {} for ship {}.", drop_id, id));
                if &drop_pos != &ship.position {
                    let towards_dropoff = &game.game_map.naive_navigate(ship, &drop_pos);
                    let command = ship.move_ship(*towards_dropoff);
                    command_queue.push(command);
                } else {
                    let command = if me.halite >= game.constants.dropoff_cost {
                        building_dropoff = false;
                        ship.make_dropoff()
                    } else {
                        ship.stay_still()
                    };
                    command_queue.push(command);
                }
                continue;

            } else if ship.halite >= game.constants.max_halite - 250 {
                ship_status.insert(id, "returning".to_string());
            }          

            let command = if halite < 10 || ship.is_full() {                
                let best_direction = game.game_map.most_halite_near_ship_direction(&ship.position);
                match best_direction {
                    Some(x) => {
                        let safe_pos = &game.game_map.naive_navigate(ship, &ship.position.directional_offset(x));
                        ship.move_ship(*safe_pos)
                    },
                    None => {                        
                        let random_direction = game.game_map.move_towards_rich_halite(&ship.position);
                        game.log.borrow_mut().log(&format!("best direction: {:?} found for ship {}.", random_direction, ship.id.0));
                        let safe_pos = &game.game_map.naive_navigate(ship, &ship.position.directional_offset(random_direction));
                        ship.move_ship(*safe_pos)
                    }
                }
            } else {
                ship.stay_still()
            };
            command_queue.push(command);
        }

        let shipyard_cell = game.game_map.at_entity(&me.shipyard);

        if game.turn_number == 60 //&&
           //me.halite < game.constants.dropoff_cost
        {
            building_dropoff = true;
            let mut best_ship = 0;
            let mut best_dropoff = 0;
            let mut min_distance = 0;
            for ship_id in &me.ship_ids {
                let ship = &game.ships[ship_id];
                for i in 0..best_dropoffs.len() {
                    let distance = game.game_map.calculate_distance(&ship.position, &best_dropoffs[i]);
                    if  distance < min_distance ||
                       min_distance == 0 {
                           best_ship = ship.id.0;
                           best_dropoff = i;
                           min_distance = distance;
                       }
                }
            }
            game.log.borrow_mut().log(&format!("ship selected dropoff: {} found for ship {}.", best_dropoff, best_ship));
            ship_status.insert(best_ship, "dropoff".to_string() + &best_dropoff.to_string());
        }

        if game.turn_number <= 200 &&
           me.halite >= game.constants.ship_cost &&
           !shipyard_cell.is_occupied() &&
           !building_dropoff
        {
            command_queue.push(me.shipyard.spawn());
        }

        Game::end_turn(&command_queue);
    }
}
