use ash::Entry;

use crate::Result;

pub fn get_vulkan_entry() -> Result<Entry> {
    unsafe { Ok(Entry::load()?) }
}
