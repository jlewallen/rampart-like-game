use super::*;

#[test]
fn test_rectangular_mapping_map_coordinates() {
    // [ 0,  1,  2,  3,  4,  5]
    // [ 6,  7,  8,  9, 10, 11]
    // [12, 13, 14, 15, 16, 17]
    // [18, 19, 20, 21, 22, 23]
    // [24, 25, 26, 27, 28, 29]
    // [30, 31, 32, 33, 34, 35]
    let data = (0..6)
        .into_iter()
        .map(|row| ((row * 6)..((row + 1) * 6)).into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let map = RectangularMapping::new(data);
    assert_eq!(
        map.map_coordinates(UVec2::new(0, 0)),
        (
            UVec2::new(0, 0),
            UVec2::new(0, 0),
            UVec2::new(0, 0),
            UVec2::new(0, 0)
        )
    );
    assert_eq!(
        map.map_coordinates(UVec2::new(1, 1)),
        (
            UVec2::new(0, 0),
            UVec2::new(1, 0),
            UVec2::new(0, 1),
            UVec2::new(1, 1)
        )
    );
    assert_eq!(
        map.map_coordinates(UVec2::new(0, 2)),
        (
            UVec2::new(0, 1),
            UVec2::new(0, 1),
            UVec2::new(0, 1),
            UVec2::new(0, 1)
        )
    );
    assert_eq!(
        map.map_coordinates(UVec2::new(5, 5)),
        (
            UVec2::new(2, 2),
            UVec2::new(3, 2),
            UVec2::new(2, 3),
            UVec2::new(3, 3)
        )
    );
}

#[test]
fn test_rectangular_mapping_map_vec_vec() {
    // [ 0,  1,  2,  3,  4,  5]
    // [ 6,  7,  8,  9, 10, 11]
    // [12, 13, 14, 15, 16, 17]
    // [18, 19, 20, 21, 22, 23]
    // [24, 25, 26, 27, 28, 29]
    // [30, 31, 32, 33, 34, 35]
    let data = (0..6)
        .into_iter()
        .map(|row| ((row * 6)..((row + 1) * 6)).into_iter().collect::<Vec<_>>())
        .collect::<Vec<_>>();

    let map = RectangularMapping::new(data);
    assert_eq!(map.get(UVec2::new(0, 0)), [0, 0, 0, 0]);
    assert_eq!(map.get(UVec2::new(1, 1)), [0, 1, 6, 7]);
    assert_eq!(map.get(UVec2::new(1, 0)), [0, 1, 0, 1]);
    assert_eq!(map.get(UVec2::new(2, 0)), [1, 1, 1, 1]);
    assert_eq!(map.get(UVec2::new(3, 0)), [1, 2, 1, 2]);
    assert_eq!(map.get(UVec2::new(4, 0)), [2, 2, 2, 2]);
    assert_eq!(map.get(UVec2::new(5, 0)), [2, 3, 2, 3]);
    assert_eq!(map.get(UVec2::new(0, 1)), [0, 0, 6, 6]);
    assert_eq!(map.get(UVec2::new(0, 2)), [6, 6, 6, 6]);
    assert_eq!(map.get(UVec2::new(0, 3)), [6, 6, 12, 12]);
    assert_eq!(map.get(UVec2::new(0, 4)), [12, 12, 12, 12]);
    assert_eq!(map.get(UVec2::new(0, 5)), [12, 12, 18, 18]);
    assert_eq!(map.get(UVec2::new(5, 5)), [14, 15, 20, 21]);
}
