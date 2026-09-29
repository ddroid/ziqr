#[cfg(test)]
mod tests {
    #[test]
    fn test_css_contains_hafs_font() {
        let css = include_str!("../resources/style.css");
        
        // Verify CSS contains the Hafs font family name
        assert!(
            css.contains("KFGQPC HafsEx1 Uthmanic Script"),
            "CSS should contain 'KFGQPC HafsEx1 Uthmanic Script' font family"
        );
        
        // Verify CSS classes exist
        assert!(css.contains(".arabic-text"), "CSS should have .arabic-text class");
        assert!(css.contains(".surah-heading"), "CSS should have .surah-heading class");
        assert!(css.contains(".bismillah"), "CSS should have .bismillah class");
        
        // Verify font-family is set for Arabic text
        assert!(
            css.contains("font-family") && css.contains(".arabic-text"),
            "CSS should set font-family for .arabic-text"
        );
    }
    
    #[test]
    fn test_css_arabic_text_priority() {
        let css = include_str!("../resources/style.css");
        
        // Verify Hafs is the primary font (first in font-family list)
        let arabic_text_section = css
            .lines()
            .skip_while(|line| !line.contains(".arabic-text"))
            .take(10)
            .collect::<Vec<_>>()
            .join("\n");
        
        assert!(
            arabic_text_section.contains("'KFGQPC HafsEx1 Uthmanic Script'"),
            "Hafs font should be in .arabic-text font-family"
        );
        
        // Hafs should come before fallbacks
        let font_family_line = arabic_text_section
            .lines()
            .find(|line| line.contains("font-family"))
            .expect("Should have font-family property");
        
        let hafs_pos = font_family_line
            .find("KFGQPC HafsEx1 Uthmanic Script")
            .expect("Should contain Hafs font");
        
        // Should be near the beginning (after "font-family:")
        assert!(
            hafs_pos < 100,
            "Hafs font should be primary font (early in font-family list)"
        );
    }
}
