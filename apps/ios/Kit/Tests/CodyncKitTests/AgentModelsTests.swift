import Foundation
import Testing
@testable import CodyncKit

@Test func modelCatalogDecodesAgentNamesAndIDs() throws {
    let data = Data(#"{"models":[{"id":"opus[1m]","name":"Agent model","description":"From the agent"}],"currentModelId":"opus[1m]"}"#.utf8)
    let catalog = try JSONDecoder().decode(AgentModels.self, from: data)
    #expect(catalog.models.first?.id == "opus[1m]")
    #expect(catalog.models.first?.name == "Agent model")
    #expect(catalog.currentModelId == "opus[1m]")
}

@Test func modelDefaultExplicitlyClearsSavedOverride() throws {
    var draft = BotDraft(name: "Example", cwd: "/tmp")
    draft.id = "bot-id"
    let data = try JSONEncoder().encode(BotUpdate(draft: draft))
    let body = try #require(JSONSerialization.jsonObject(with: data) as? [String: Any])
    #expect(body["model"] is NSNull)
    #expect(body["id"] as? String == "bot-id")
    #expect(body["name"] as? String == "Example")
    draft.model = "agent-model"
    let selected = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(BotUpdate(draft: draft))) as? [String: Any])
    #expect(selected["model"] as? String == "agent-model")
}

@Test func modelCatalogReportsFastModeAndOlderHostsOmitIt() throws {
    let offered = try JSONDecoder().decode(AgentModels.self, from: Data(#"{"models":[],"fastMode":true}"#.utf8))
    #expect(offered.fastMode == true)
    let older = try JSONDecoder().decode(AgentModels.self, from: Data(#"{"models":[]}"#.utf8))
    #expect(older.fastMode == nil)
}

@Test func botFastModeReachesTheSettingsDraft() throws {
    let bot = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"b","backend":"codex","fastMode":true}"#.utf8))
    #expect(bot.fastMode)
    let draft = BotDraft(bot)
    let body = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(BotUpdate(draft: draft))) as? [String: Any])
    #expect(body["fastMode"] as? Bool == true)
    let older = try JSONDecoder().decode(Bot.self, from: Data(#"{"id":"b"}"#.utf8))
    #expect(!older.fastMode)
}

@Test func composerOffersFastModeOnceTheAgentDoes() throws {
    func bot(_ json: String) throws -> Bot { try JSONDecoder().decode(Bot.self, from: Data(json.utf8)) }
    #expect(try !bot(#"{"id":"b"}"#).fastModeShown)
    #expect(try bot(#"{"id":"b","fastModeSupport":"offered"}"#).fastModeShown)
    #expect(try bot(#"{"id":"b","fastMode":true,"fastModeSupport":"unsupported"}"#).fastModeShown)
    #expect(try !bot(#"{"id":"g","kind":"group","fastModeSupport":"offered"}"#).fastModeShown)
    let refused = try bot(#"{"id":"b","fastMode":true,"fastModeSupport":"refused","fastModeIssue":"requires extra usage"}"#)
    #expect(refused.fastModeState == .refused)
    #expect(refused.fastModeRefusal?.contains("requires extra usage") == true)
}
