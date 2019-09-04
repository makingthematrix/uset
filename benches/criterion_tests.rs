#[macro_use]
extern crate criterion;

use criterion::Criterion;

fn gen_uset(c: &mut Criterion) {
    c.bench_function("USet generate map 1000", |b| {
        b.iter({ || gen_cities_uset(1000, 75) })
    });
}

fn gen_hashset(c: &mut Criterion) {
    c.bench_function("HashSet generate map 1000", |b| {
        b.iter({ || gen_cities_hashset(1000, 75) })
    });
}

fn solve(c: &mut Criterion) {
    let map = gen_cities_uset(1000, 75);
    c.bench_function("Solve map 1000", move |b| {
        b.iter({ || find_city_distances(&map) })
    });
}

criterion_group!(benches, gen_uset, gen_hashset, solve);
criterion_main!(benches);

// ---

use std;
use std::cmp::min;

extern crate rand;

use rand::*;

use std::collections::HashSet;
use uset::core::uset::USet;

/// Calculates a vector where indexes are the distances from the capital and the values are
/// the number of cities with the given distance.
///
/// # Arguments
///
/// * `city_vec` - A vector describing the map of cities.
///   `city_vec[x] == y` means that there is a road from the city `x` to the city `y`.
///   We assume there are no cycles (ie. the map is a tree) and there is exactly one "capital" such
///   that `city_vec[capital] == capital`.
///
/// # Example
///
/// ```
/// let city_vec = vec![9, 3, 2, 2, 2, 2, 1, 5, 2, 1];
/// let distances = find_city_distances(&city_vec);
/// assert_eq!(distances, vec![1, 4, 2, 2, 1]);
/// ```
pub fn find_city_distances(city_vec: &[usize]) -> Vec<usize> {
    let (capital, ..) = city_vec.iter().enumerate().find(|&(i, &c)| i == c).unwrap();
    let mut distance_map = vec![std::usize::MAX; city_vec.len()];
    distance_map[capital] = 0;

    for i in 0..city_vec.len() {
        update_distance_map(i, city_vec, &mut distance_map);
    }

    let &max_distance = distance_map.iter().max().unwrap();
    let result: Vec<usize> =
        distance_map
            .iter()
            .fold(vec![0; max_distance + 1], |mut acc, &distance| {
                acc[distance] += 1;
                acc
            });

    result
}

fn update_distance_map(i: usize, city_vec: &[usize], distance_map: &mut [usize]) {
    let mut path = Vec::new();
    let found_distance = find_path(i, city_vec, distance_map, &mut path);
    path.iter()
        .enumerate()
        .for_each(|(path_index, &city_index)| {
            distance_map[city_index] = found_distance + path.len() - path_index
        })
}

fn find_path(
    i: usize,
    city_array: &[usize],
    distance_map: &[usize],
    path: &mut Vec<usize>,
) -> usize {
    if distance_map[i] == std::usize::MAX {
        path.push(i);
        find_path(city_array[i], city_array, distance_map, path) // should be tailrec
    } else {
        distance_map[i]
    }
}

fn gen_unshuffled(
    size: usize,
    max_broad: usize,
    knots_occurence: f32,
    knots_max_broad: usize,
) -> Vec<usize> {
    debug_assert!(size > 0);
    debug_assert!(max_broad <= size);
    debug_assert!(knots_occurence >= 0.0);
    debug_assert!(knots_occurence <= 1.0);
    debug_assert!(knots_max_broad >= max_broad);

    let mut city_vec = Vec::with_capacity(size);
    let mut r = rand::thread_rng();

    city_vec.push(0);

    let mut index: usize = 0;
    while city_vec.len() < size {
        let min_broad = if index == city_vec.len() - 1 { 1 } else { 0 };
        let b = if r.gen_range(0.0, 1.0) < knots_occurence {
            knots_max_broad
        } else {
            max_broad
        };

        let max_b = min(b, size - city_vec.len());
        let s = if min_broad < max_b {
            r.gen_range(min_broad, max_b)
        } else {
            min_broad
        };

        for _i in 0..s {
            city_vec.push(index);
        }

        index += 1;
    }
    city_vec
}

fn city_swap(city_vec: &mut Vec<usize>, from: usize, to: usize) {
    city_vec.swap(from, to);
    for v in city_vec.iter_mut().skip(1) {
        if *v == from {
            *v = to
        } else if *v == to {
            *v = from
        };
    }
}

fn shuffle_cities(mut city_vec: &mut Vec<usize>) {
    debug_assert!(city_vec.len() > 1);
    let mut r = rand::thread_rng();

    for _i in 1..(city_vec.len() / 2 - 1) {
        let from = r.gen_range(1, city_vec.len());
        let to = r.gen_range(1, city_vec.len());
        if from != to {
            city_swap(&mut city_vec, from, to);
        }
    }

    let capital_switch = r.gen_range(1, city_vec.len());
    city_swap(&mut city_vec, 0, capital_switch);
}

/// Generates a city map.
///
/// By default creates a very "round map" with every city having roads to on average the same number
/// of other cities. As the result, the computed distances vector is short and with values growing
/// with each index until the penultimate. To make the map more interesting, the user may set
/// `max_roads` to a very low number and instead allow for big "travel centres" with larger number
/// of roads.
///
/// # Arguments
/// * `size` - the total number of cities
/// * `max_roads` - the maximum number of roads - 1 from one city to others
/// * `travel_centre_possibility` - the probability of the city having more than `max_roads`
/// * `centre_max_roads` - the maximum number of roads - 1 for the travel centre
///
/// # Example
///
/// ```
/// let city_vec = gen_cities(10, 2, 0.2, 4);
/// assert_eq!(city_vec.len(), 10);
/// ```
pub fn gen_cities(
    size: usize,
    max_roads: usize,
    travel_centre_possibility: f32,
    centre_max_roads: usize,
) -> Vec<usize> {
    let mut city_vec = gen_unshuffled(size, max_roads, travel_centre_possibility, centre_max_roads);
    shuffle_cities(&mut city_vec);
    city_vec
}

/// Generates a city map.
///
/// Creates more "snaky" maps than `gen_cities`. The computed distances vectors tend to be long and
/// with low values at each index. Slower, and more complex. Uses `USet`.
///
/// # Arguments
/// * `size` - the total number of cities
/// * `max_roads_per_distance` - the maximum number of roads leading to *all* cities with the same
///   distance to the capital
///
/// # Example
///
/// ```
/// let city_vec = gen_cities_uset(10, 3);
/// assert_eq!(city_vec.len(), 10);
/// ```
pub fn gen_cities_uset(size: usize, max_roads_per_distance: usize) -> Vec<usize> {
    let mut city_vec = Vec::with_capacity(size);
    let mut r = rand::thread_rng();

    let all_cities = USet::from(0..size);
    let capital = r.gen_range(0, size);
    city_vec.push((capital, capital));

    while city_vec.len() < size {
        let (city, ..) = city_vec[city_vec.len() - 1];
        let high = min(max_roads_per_distance, size - city_vec.len());
        let new_cities = r.gen_range(0, high) + 1;

        let used_cities: USet = city_vec.iter().map(|&(x, _)| x).collect();
        let mut free_cities = &all_cities - &used_cities;
        let max_cities = min(new_cities, free_cities.len());

        for _i in 0..max_cities {
            let new_city = pop_random(&mut free_cities).unwrap();
            city_vec.push((new_city, city));
        }
    }

    let mut city_array = vec![0; size];
    city_vec
        .iter()
        .for_each(|&(from, to)| city_array[from] = to);
    city_array
}

fn pop_random(set: &mut USet) -> Option<usize> {
    if !set.is_empty() {
        let index = rand::thread_rng().gen_range(0, set.len());
        set.pop(index)
    } else {
        None
    }
}

/// Generates a city map.
///
/// Same as `gen_cities_uset` but uses `std::collections::HashSet` instead of `USet`.
/// Implemented for performance comparison.
pub fn gen_cities_hashset(size: usize, max_roads_per_distance: usize) -> Vec<usize> {
    let mut city_vec = Vec::with_capacity(size);
    let mut r = rand::thread_rng();

    let all_cities: HashSet<usize> = (0..size).collect();

    let capital = r.gen_range(0, size);
    city_vec.push((capital, capital));

    while city_vec.len() < size {
        let (city, ..) = city_vec[city_vec.len() - 1];
        let high = min(max_roads_per_distance, size - city_vec.len());
        let new_cities = r.gen_range(0, high) + 1;

        let used_cities: HashSet<usize> = city_vec.iter().map(|&(x, _)| x).collect();
        let mut free_cities = &all_cities - &used_cities;
        let max_cities = min(new_cities, free_cities.len());

        for _i in 0..max_cities {
            let remove_index = r.gen_range(0, free_cities.len());
            let &new_city = free_cities.iter().nth(remove_index).unwrap();
            free_cities.remove(&new_city);
            city_vec.push((new_city, city));
        }
    }

    let mut city_array = vec![0; size];
    city_vec
        .iter()
        .for_each(|&(from, to)| city_array[from] = to);
    city_array
}
