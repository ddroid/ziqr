use std::path::PathBuf;

/// Test that Hafs font file is properly installed in user's font directory.
#[test]
fn test_hafs_font_installed() {
    let font_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fonts");
    
    let font_path = font_dir.join("Hafs.ttf");
    
    // Verify font file exists
    assert!(
        font_path.exists(),
        "Hafs.ttf not found at {:?}",
        font_path
    );
    
    // Verify font file is not empty
    let metadata = std::fs::metadata(&font_path).expect("Cannot read font metadata");
    assert!(
        metadata.len() > 10000,
        "Hafs.ttf is too small ({} bytes), might be corrupted",
        metadata.len()
    );
    
    eprintln!("✓ Hafs font installed at: {:?}", font_path);
    eprintln!("✓ Font file size: {} bytes", metadata.len());
}

/// Test that CSS correctly references Hafs font.
#[test]
fn test_css_references_hafs_font() {
    let css = include_str!("../resources/style.css");
    
    // Verify CSS contains the correct Hafs font family name
    assert!(
        css.contains("KFGQPC HafsEx1 Uthmanic Script"),
        "CSS must contain 'KFGQPC HafsEx1 Uthmanic Script' font family"
    );
    
    // Verify .arabic-text class exists
    assert!(
        css.contains(".arabic-text"),
        "CSS must have .arabic-text class"
    );
    
    // Verify .arabic-text uses Hafs font
    let arabic_section = css
        .lines()
        .skip_while(|line| !line.contains(".arabic-text"))
        .take(10)
        .collect::<Vec<_>>()
        .join("\n");
    
    assert!(
        arabic_section.contains("font-family") && arabic_section.contains("KFGQPC HafsEx1 Uthmanic Script"),
        ".arabic-text must use KFGQPC HafsEx1 Uthmanic Script font"
    );
    
    // Verify Hafs is the primary font (first in font-family list)
    let font_family_line = arabic_section
        .lines()
        .find(|line| line.contains("font-family"))
        .expect(".arabic-text must have font-family property");
    
    let hafs_pos = font_family_line
        .find("KFGQPC HafsEx1 Uthmanic Script")
        .expect("font-family must contain Hafs font");
    
    assert!(
        hafs_pos < 50,
        "Hafs font should be primary (first in font-family list)"
    );
    
    eprintln!("✓ CSS correctly references Hafs font");
    eprintln!("✓ .arabic-text class exists");
    eprintln!("✓ Hafs is primary font in .arabic-text");
}
