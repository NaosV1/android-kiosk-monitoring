package fr.tablette.kiosque;

import android.app.admin.DevicePolicyManager;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;

/** Kiosk policy helpers. Constants are literal on purpose (values from the Android SDK). */
final class Kiosk {
    static final String PKG = "fr.tablette.kiosque";
    static final String X11_PKG = "com.termux.x11";
    static final String X11_ACTIVITY = "com.termux.x11.MainActivity";
    /** Apps allowed to run while the tablet is locked. */
    static final String[] LOCK_TASK_PKGS = {PKG, X11_PKG, "com.termux", "com.termux.boot"};

    static final int FLAG_ACTIVITY_NEW_TASK = 0x10000000;
    static final int LOCK_TASK_FEATURE_NONE = 0;
    static final int COMPONENT_ENABLED = 1;   // PackageManager.COMPONENT_ENABLED_STATE_ENABLED
    static final int COMPONENT_DISABLED = 2;  // PackageManager.COMPONENT_ENABLED_STATE_DISABLED
    static final int DONT_KILL_APP = 1;       // PackageManager.DONT_KILL_APP

    private Kiosk() {}

    static ComponentName admin(Context c) { return new ComponentName(PKG, PKG + ".Admin"); }
    static ComponentName homeAlias() { return new ComponentName(PKG, PKG + ".HomeAlias"); }

    static DevicePolicyManager dpm(Context c) {
        return (DevicePolicyManager) c.getSystemService("device_policy");
    }

    static boolean isOwner(Context c) { return dpm(c).isDeviceOwnerApp(PKG); }

    static SharedPreferences prefs(Context c) { return c.getSharedPreferences("kiosque", 0); }

    static boolean enabled(Context c) { return prefs(c).getBoolean("enabled", true); }

    static void setEnabled(Context c, boolean on) {
        prefs(c).edit().putBoolean("enabled", on).commit();
    }

    static void setHomeAlias(Context c, boolean on) {
        c.getPackageManager().setComponentEnabledSetting(homeAlias(),
                on ? COMPONENT_ENABLED : COMPONENT_DISABLED, DONT_KILL_APP);
    }

    /** Lock: only Linux display allowed, no home/recents/notifications/power menu, we are the launcher. */
    static void applyLock(Context c) {
        if (!isOwner(c)) return;
        DevicePolicyManager d = dpm(c);
        ComponentName a = admin(c);
        d.setLockTaskPackages(a, LOCK_TASK_PKGS);
        d.setLockTaskFeatures(a, LOCK_TASK_FEATURE_NONE);
        d.setStatusBarDisabled(a, true);
        d.setKeyguardDisabled(a, true);
        IntentFilter f = new IntentFilter("android.intent.action.MAIN");
        f.addCategory("android.intent.category.HOME");
        f.addCategory("android.intent.category.DEFAULT");
        d.addPersistentPreferredActivity(a, f, homeAlias());
    }

    /** Unlock: give Android back (launcher, status bar), leave lock task mode. */
    static void releaseLock(Context c) {
        if (!isOwner(c)) return;
        DevicePolicyManager d = dpm(c);
        ComponentName a = admin(c);
        d.clearPackagePersistentPreferredActivities(a, PKG);
        d.setStatusBarDisabled(a, false);
        d.setKeyguardDisabled(a, false);
        d.setLockTaskPackages(a, new String[0]); // ends lock task mode
    }

    static void launchX11(Context c) {
        Intent i = new Intent();
        i.setComponent(new ComponentName(X11_PKG, X11_ACTIVITY));
        i.addFlags(FLAG_ACTIVITY_NEW_TASK);
        try {
            c.startActivity(i);
        } catch (RuntimeException e) {
            // Termux:X11 missing: stay on our black screen
        }
    }

    static void goHome(Context c) {
        Intent h = new Intent("android.intent.action.MAIN");
        h.addCategory("android.intent.category.HOME");
        h.addFlags(FLAG_ACTIVITY_NEW_TASK);
        try {
            c.startActivity(h);
        } catch (RuntimeException e) {
            // ignore
        }
    }
}
