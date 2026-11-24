package fr.tablette.kiosque;

import android.app.admin.DevicePolicyManager;
import android.content.BroadcastReceiver;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;

/**
 * Remote control, called from Termux or adb:
 *   am broadcast -n fr.tablette.kiosque/.Control -a fr.tablette.kiosque.UNLOCK --es token TOKEN
 * Actions: LOCK, UNLOCK, STATUS, RELEASE (removes device owner, Android fully back).
 */
public class Control extends BroadcastReceiver {
    private static final String TOKEN = "@TOKEN@";
    private static final String A = "fr.tablette.kiosque.";

    @Override
    public void onReceive(Context c, Intent intent) {
        if (!TOKEN.equals(intent.getStringExtra("token"))) {
            setResultData("refuse: token invalide");
            return;
        }
        String action = intent.getAction();
        if ((A + "UNLOCK").equals(action)) {
            Kiosk.setEnabled(c, false);
            Kiosk.releaseLock(c);
            Kiosk.setHomeAlias(c, false);
            Kiosk.goHome(c);
            setResultData("deverrouille");
        } else if ((A + "LOCK").equals(action)) {
            Kiosk.setEnabled(c, true);
            Kiosk.setHomeAlias(c, true);
            Kiosk.applyLock(c);
            Intent h = new Intent();
            h.setComponent(new ComponentName(Kiosk.PKG, Kiosk.PKG + ".Home"));
            h.addFlags(Kiosk.FLAG_ACTIVITY_NEW_TASK);
            c.startActivity(h);
            setResultData("verrouille");
        } else if ((A + "RELEASE").equals(action)) {
            Kiosk.setEnabled(c, false);
            Kiosk.releaseLock(c);
            Kiosk.setHomeAlias(c, false);
            DevicePolicyManager d = Kiosk.dpm(c);
            if (Kiosk.isOwner(c)) {
                d.clearDeviceOwnerApp(Kiosk.PKG);
            }
            Kiosk.goHome(c);
            setResultData("proprietaire retire");
        } else {
            setResultData("proprietaire=" + Kiosk.isOwner(c) + " verrouillage=" + Kiosk.enabled(c));
        }
    }
}
