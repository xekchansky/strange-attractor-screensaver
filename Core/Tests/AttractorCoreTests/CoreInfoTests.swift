import Testing
@testable import AttractorCore

@Suite("CoreInfo")
struct CoreInfoTests {
    @Test("Module name is stable")
    func moduleNameIsStable() {
        #expect(CoreInfo.moduleName == "AttractorCore")
    }

    @Test("API version is semver")
    func apiVersionIsSemver() {
        let parts = CoreInfo.apiVersion.split(separator: ".")
        #expect(parts.count == 3)
        #expect(parts.allSatisfy { Int($0) != nil })
    }
}
