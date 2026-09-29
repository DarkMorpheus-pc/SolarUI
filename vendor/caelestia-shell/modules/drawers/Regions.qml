pragma ComponentBehavior: Bound

import QtQuick
import Quickshell
import Caelestia.Config
import qs.modules.bar as Bar

Region {
    id: root

    required property Bar.BarWrapper bar
    required property Panels panels
    required property var win

    readonly property real borderThickness: win.contentItem.Config.border.thickness
    readonly property real clampedThickness: 0 // Set to 0 so Caelestia border hit-mask does not block top 16px (Noctalia & window close/minimize buttons)
    readonly property real edgeThickness: Math.min(3, root.borderThickness)

    x: bar.clampedWidth + win.dragMaskPadding
    y: clampedThickness + win.dragMaskPadding
    width: win.width - bar.clampedWidth - clampedThickness - win.dragMaskPadding * 2
    height: win.height - clampedThickness * 2 - win.dragMaskPadding * 2
    intersection: Intersection.Xor

    R {
        panel: root.panels.dashboard
        y: 0
        height: root.panels.dashboard.offsetScale < 1 ? (panel.height * (1 - root.panels.dashboard.offsetScale) + root.borderThickness) : root.edgeThickness
    }

    R {
        panel: root.panels.launcher
        y: root.win.height - height
        height: root.panels.launcher.offsetScale < 1 ? (panel.height * (1 - root.panels.launcher.offsetScale) + root.borderThickness) : root.edgeThickness
    }

    R {
        id: sessionRegion

        panel: root.panels.sessionWrapper
        x: root.win.width - width
        width: root.panels.session.offsetScale < 1 ? (panel.width * (1 - root.panels.session.offsetScale) + root.borderThickness + sidebarRegion.width) : root.edgeThickness
    }

    R {
        id: sidebarRegion

        panel: root.panels.sidebar
        x: root.win.width - width
        width: root.panels.sidebar.offsetScale < 1 ? (panel.width * (1 - root.panels.sidebar.offsetScale) + root.borderThickness) : root.edgeThickness
    }

    R {
        panel: root.panels.osdWrapper
        x: root.win.width - width
        width: root.panels.osd.offsetScale < 1 ? (panel.width * (1 - root.panels.osd.offsetScale) + root.borderThickness + sessionRegion.width) : root.edgeThickness
    }

    R {
        panel: root.panels.notifications
        y: 0
        height: panel.height > 0 ? (panel.height + root.borderThickness) : 0
    }

    R {
        panel: root.panels.utilities
        y: root.win.height - height
        height: root.panels.utilities.offsetScale < 1 ? (panel.height * (1 - root.panels.utilities.offsetScale) + root.borderThickness) : root.edgeThickness
    }

    R {
        panel: root.panels.popoutsWrapper
        width: panel.width * (1 - root.panels.popoutsWrapper.offsetScale)
    }

    Region {
        // Exclude Top Bar area so Noctalia Bar receives mouse clicks
        x: 0
        y: 0
        width: root.win.width
        height: 40
        intersection: Intersection.Subtract
    }

    Region {
        // Exclude Bottom Taskbar area so Taskbar receives mouse clicks
        x: 0
        y: root.win.height - 48
        width: root.win.width
        height: 48
        intersection: Intersection.Subtract
    }

    component R: Region {
        required property Item panel

        x: panel.x + root.bar.implicitWidth
        y: panel.y + root.borderThickness
        width: panel.width
        height: panel.height
        intersection: Intersection.Subtract
    }
}
