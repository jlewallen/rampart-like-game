use bevy::math::UVec2;

use crate::{model::Around, model::AroundCenter};

use super::*;

#[test]
fn test_around_square_grid() {
    let grid: SquareGrid<u32> = SquareGrid::new_flat(UVec2::new(64, 64));
    assert_eq!(grid.get_xy(IVec2::new(63, 63)), Some(&0));
    assert_eq!(grid.get_xy(IVec2::new(64, 64)), None);
    assert_eq!(grid.get_xy(IVec2::new(-1, -1)), None);
    assert_eq!(
        grid.around(IVec2::new(0, 0)),
        Around::new((
            (None, None, None),
            (None, Some(0), Some(0)),
            (None, Some(0), Some(0))
        ))
    );
    assert_eq!(
        grid.around(IVec2::new(63, 63)),
        Around::new((
            (Some(0), Some(0), None),
            (Some(0), Some(0), None),
            (None, None, None),
        ))
    );
}
