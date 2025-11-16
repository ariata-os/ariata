// Version information for ariata-mac
struct Version {
    static let current = "0.0.2"
    static let buildDate = "2025-08-17"
    static let gitCommit = "unknown" // Will be set during CI build
    
    static var full: String {
        return "\(current) (\(buildDate))"
    }
    
    static var userAgent: String {
        return "ariata-mac/\(current)"
    }
}