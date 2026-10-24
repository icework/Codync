import CodyncKit
import CodyncUI

extension AppStore {
    func openShortcutBot(_ bot: ShortcutBotEntity) throws -> BotStore {
        guard let ref = bot.reference(in: storage), let store = accounts.store(for: ref.computerId) else {
            throw HostError.http(404, "This bot isn't available in the current account. Open Codync and choose a bot again in the shortcut.")
        }
        for store in accounts.stores.values { store.screenRequest = nil }
        account.showSwitcher = false
        showComputers = false
        showUsage = false
        marketplace = nil
        tab = .bots
        accounts.selection = ref
        accounts.setActive(true)
        return store
    }
}
