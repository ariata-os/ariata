import Cocoa

class EmojiPickerViewController: NSViewController {

    var onEmojiSelected: ((String) -> Void)?

    private let emojis = [
        // Primary Tech (8)
        "📡", "🚀", "💡", "⭐", "🔥", "🎯", "🧠", "💻",
        // Status Colors (8)
        "🟢", "🔵", "🟡", "🔴", "⚫", "🟣", "🟠", "🟥",
        // Fun/Creative (8)
        "🎉", "✨", "🎨", "🎭", "🎪", "🎸", "🎬", "🎲"
    ]

    override func loadView() {
        // Create container view - 6 columns × 4 rows with spacing and padding
        let buttonSize: CGFloat = 36
        let spacing: CGFloat = 4
        let padding: CGFloat = 8
        let columns = 6
        let rows = 4

        let width = (buttonSize * CGFloat(columns)) + (spacing * CGFloat(columns - 1)) + (padding * 2)
        let height = (buttonSize * CGFloat(rows)) + (spacing * CGFloat(rows - 1)) + (padding * 2)

        let containerView = NSView(frame: NSRect(x: 0, y: 0, width: width, height: height))
        self.view = containerView

        // Create grid of emoji buttons
        for (index, emoji) in emojis.enumerated() {
            let row = index / columns
            let col = index % columns

            let x = padding + CGFloat(col) * (buttonSize + spacing)
            let y = padding + CGFloat(3 - row) * (buttonSize + spacing)  // Flip Y coordinate

            let button = NSButton(frame: NSRect(x: x, y: y, width: buttonSize, height: buttonSize))
            button.title = emoji
            button.bezelStyle = .recessed
            button.isBordered = true
            button.font = NSFont.systemFont(ofSize: 24)
            button.target = self
            button.action = #selector(emojiButtonClicked(_:))
            button.tag = index

            // Make button transparent and remove state effects
            button.wantsLayer = true
            button.layer?.backgroundColor = NSColor.clear.cgColor
            button.layer?.cornerRadius = 6
            button.contentTintColor = nil  // Don't tint the emoji

            containerView.addSubview(button)
        }
    }

    @objc private func emojiButtonClicked(_ sender: NSButton) {
        let emoji = emojis[sender.tag]
        onEmojiSelected?(emoji)
    }
}
