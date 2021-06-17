mod alloc;

fn main() -> Result<(), std::io::Error> {
    // To strop marking everything as unused code for now
    alloc::get_page().ok();

    Ok(())
}
