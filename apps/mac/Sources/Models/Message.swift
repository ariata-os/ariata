import Foundation

struct Message {
    let id: Int64?
    let messageId: String
    let chatId: String
    let handleId: String?
    let text: String?
    let service: String
    let isFromMe: Bool
    let date: Date
    let dateRead: Date?
    let dateDelivered: Date?
    let isRead: Bool
    let isDelivered: Bool
    let isSent: Bool
    let cacheHasAttachments: Bool
    let attachmentCount: Int?
    let attachmentInfo: [[String: Any]]?
    let groupTitle: String?
    let associatedMessageGuid: String?
    let associatedMessageType: Int?
    let expressiveSendStyleId: String?
    var uploaded: Bool = false
    
    init(
        messageId: String,
        chatId: String,
        handleId: String? = nil,
        text: String? = nil,
        service: String = "iMessage",
        isFromMe: Bool,
        date: Date,
        dateRead: Date? = nil,
        dateDelivered: Date? = nil,
        isRead: Bool = false,
        isDelivered: Bool = false,
        isSent: Bool = false,
        cacheHasAttachments: Bool = false,
        attachmentCount: Int? = nil,
        attachmentInfo: [[String: Any]]? = nil,
        groupTitle: String? = nil,
        associatedMessageGuid: String? = nil,
        associatedMessageType: Int? = nil,
        expressiveSendStyleId: String? = nil
    ) {
        self.id = nil
        self.messageId = messageId
        self.chatId = chatId
        self.handleId = handleId
        self.text = text
        self.service = service
        self.isFromMe = isFromMe
        self.date = date
        self.dateRead = dateRead
        self.dateDelivered = dateDelivered
        self.isRead = isRead
        self.isDelivered = isDelivered
        self.isSent = isSent
        self.cacheHasAttachments = cacheHasAttachments
        self.attachmentCount = attachmentCount
        self.attachmentInfo = attachmentInfo
        self.groupTitle = groupTitle
        self.associatedMessageGuid = associatedMessageGuid
        self.associatedMessageType = associatedMessageType
        self.expressiveSendStyleId = expressiveSendStyleId
        self.uploaded = false
    }
    
    var toDictionary: [String: Any] {
        var dict: [String: Any] = [
            "message_id": messageId,
            "chat_id": chatId,
            "is_from_me": isFromMe,
            "date": ISO8601DateFormatter().string(from: date),
            "service": service,
            "is_read": isRead,
            "is_delivered": isDelivered,
            "is_sent": isSent,
            "cache_has_attachments": cacheHasAttachments
        ]
        
        if let handleId = handleId {
            dict["handle_id"] = handleId
        }
        
        if let text = text {
            dict["text"] = text
        }
        
        if let dateRead = dateRead {
            dict["date_read"] = ISO8601DateFormatter().string(from: dateRead)
        }
        
        if let dateDelivered = dateDelivered {
            dict["date_delivered"] = ISO8601DateFormatter().string(from: dateDelivered)
        }
        
        if let attachmentCount = attachmentCount {
            dict["attachment_count"] = attachmentCount
        }
        
        if let attachmentInfo = attachmentInfo {
            dict["attachment_info"] = attachmentInfo
        }
        
        if let groupTitle = groupTitle {
            dict["group_title"] = groupTitle
        }
        
        if let associatedMessageGuid = associatedMessageGuid {
            dict["associated_message_guid"] = associatedMessageGuid
        }
        
        if let associatedMessageType = associatedMessageType {
            dict["associated_message_type"] = associatedMessageType
        }
        
        if let expressiveSendStyleId = expressiveSendStyleId {
            dict["expressive_send_style_id"] = expressiveSendStyleId
        }
        
        return dict
    }
    
    // Convert from macOS Core Data timestamp (nanoseconds since 2001-01-01)
    static func dateFromCoreDataTimestamp(_ timestamp: Double) -> Date {
        // Core Data epoch: 2001-01-01 00:00:00 UTC
        // Unix epoch: 1970-01-01 00:00:00 UTC
        // Difference: 978307200 seconds
        let coreDataEpochOffset: TimeInterval = 978307200
        
        // Messages.app uses nanoseconds since 2001-01-01
        // Check if this is a nanosecond timestamp (very large number)
        if timestamp > 1_000_000_000_000 {
            // Convert nanoseconds to seconds
            let secondsSince2001 = timestamp / 1_000_000_000
            return Date(timeIntervalSince1970: secondsSince2001 + coreDataEpochOffset)
        } else if timestamp < 1_000_000_000 {
            // It's already in seconds since 2001
            return Date(timeIntervalSince1970: timestamp + coreDataEpochOffset)
        } else {
            // It's a Unix timestamp in seconds
            return Date(timeIntervalSince1970: timestamp)
        }
    }
}