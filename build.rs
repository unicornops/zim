//! Embed the Windows file version used by the MSI installer.

fn main() -> std::io::Result<()> {
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set("ProductName", "Zim")
        .set("FileDescription", "Zim desktop instant messaging client")
        .compile()?;

    Ok(())
}
