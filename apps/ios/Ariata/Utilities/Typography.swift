//
//  Typography.swift
//  Ariata
//
//  Typography modifiers for consistent text styling
//

import SwiftUI

// MARK: - Typography View Modifiers

struct H1Style: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(.system(.largeTitle, design: .serif))
            .fontWeight(.bold)
    }
}

struct H2Style: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(.system(.title2, design: .serif))
            .fontWeight(.bold)
    }
}

// MARK: - View Extensions

extension View {
    /// Applies h1 style with serif font (large title, bold)
    func h1Style() -> some View {
        self.modifier(H1Style())
    }

    /// Applies h2 style with serif font (title2, bold)
    func h2Style() -> some View {
        self.modifier(H2Style())
    }
}
