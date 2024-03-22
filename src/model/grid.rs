use bevy::math::*;

pub trait XyIndex<T> {
    fn get_xy(&self, p: IVec2) -> Option<&T>;
}
