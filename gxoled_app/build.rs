use std::io;

fn main() -> io::Result<()> {
    #[cfg(target_os = "windows")]
    winresource::WindowsResource::new()
        .set_icon("assets/gxoled.ico")
        .set("ProductName", "GxOled")
        .set("FileDescription", "GxOled - SteelSeries Base Station OLED")
        .set("InternalName", "GxOled")
        .set("OriginalFilename", "gxoled.exe")
        .compile()?;
    Ok(())
}
