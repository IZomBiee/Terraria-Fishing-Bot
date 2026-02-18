use crate::opencv;

pub struct Detector {}

impl Detector {
    pub fn new() -> Detector {
        Detector {}
    }

    pub fn get_bobber_pos(&self, mask: &[u8], width: u32, height: u32) -> Option<(u32, u32)> {
        let mut consumed_mask = mask.to_vec();
        let blobs = opencv::find_blobs(&mut consumed_mask, width, height, 10);

        if let Some(smallest_blob) = blobs.iter().min_by_key(|blob| -> u32 { blob.area }) {
            return Some((smallest_blob.center_x, smallest_blob.center_y));
        };

        None
    }

    pub fn biting_name(&self) -> Option<String> {
        None
    }
}
