pragma Singleton
import qs
import qs.modules.common
import qs.modules.common.functions
import QtQuick
import Quickshell
import Quickshell.Io

/*
 * System updates service. Supports Portadora (BlazeOS), DNF/Fedora, and Arch checkupdates.
 */
Singleton {
    id: root

    property bool available: false
    property alias checking: checkUpdatesProc.running
    property int count: 0
    
    readonly property bool updateAdvised: available && count > Config.options.updates.adviseUpdateThreshold
    readonly property bool updateStronglyAdvised: available && count > Config.options.updates.stronglyAdviseUpdateThreshold

    function load() {}
    function refresh() {
        if (!available) return;
        print("[Updates] Checking for system updates")
        checkUpdatesProc.running = true;
    }

    Timer {
        interval: Config.options.updates.checkInterval * 60 * 1000
        repeat: true
        running: Config.ready && Config.options.updates.enableCheck
        onTriggered: {
            print("[Updates] Periodic update check due")
            root.refresh();
        }
    }

    Process {
        id: checkAvailabilityProc
        running: Config.ready && Config.options.updates.enableCheck
        command: ["bash", "-c", "command -v portadora >/dev/null 2>&1 || command -v checkupdates >/dev/null 2>&1 || command -v dnf >/dev/null 2>&1"]
        onExited: (exitCode, exitStatus) => {
            root.available = (exitCode === 0);
            firstCheckTimer.start();
        }
    }

    Timer {
        id: firstCheckTimer
        interval: 60 * 1000
        onTriggered: root.refresh()
    }

    Process {
        id: checkUpdatesProc
        command: ["bash", "-c", "if command -v portadora >/dev/null 2>&1; then echo 0; elif command -v checkupdates >/dev/null 2>&1; then pacman=$(checkupdates 2>/dev/null | wc -l); if command -v yay >/dev/null 2>&1; then aur=$(yay -Qua 2>/dev/null | wc -l); elif command -v paru >/dev/null 2>&1; then aur=$(paru -Qua 2>/dev/null | wc -l); else aur=0; fi; echo $((pacman + aur)); elif command -v dnf >/dev/null 2>&1; then dnf -q check-update 2>/dev/null | awk 'NF >= 3 && $1 ~ /\\./ { count++ } END { print count + 0 }'; else echo 0; fi"]
        stdout: StdioCollector {
            onStreamFinished: {
                const parsed = parseInt(text.trim());
                root.count = Number.isNaN(parsed) ? 0 : parsed;
            }
        }
    }
}
