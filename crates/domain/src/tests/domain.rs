use super::*;

#[test]
fn rejects_coordinates_outside_wgs84_bounds() {
    assert!(
        Coordinates {
            latitude: -33.8688,
            longitude: 151.2093
        }
        .is_valid()
    );
    assert!(
        !Coordinates {
            latitude: 91.0,
            longitude: 0.0
        }
        .is_valid()
    );
}
