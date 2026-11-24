package fr.tablette.kiosque;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

/** Launcher while locked: enters lock task mode and keeps the Linux display (Termux:X11) in front. */
public class Home extends Activity {
    private TextView text;

    @Override
    protected void onCreate(Bundle state) {
        super.onCreate(state);
        text = new TextView(this);
        text.setTextSize(22f);
        text.setGravity(17); // Gravity.CENTER
        text.setTextColor(0xFFFFFFFF);
        text.setBackgroundColor(0xFF000000);
        setContentView(text);
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (Kiosk.isOwner(this) && Kiosk.enabled(this)) {
            Kiosk.applyLock(this);
            try {
                startLockTask();
            } catch (RuntimeException e) {
                // already locked or not allowed
            }
            text.setText("Demarrage de Linux...");
            text.postDelayed(new Runnable() {
                @Override
                public void run() {
                    Kiosk.launchX11(Home.this);
                }
            }, 1500);
        } else if (!Kiosk.isOwner(this)) {
            text.setText("Kiosque : cette appli n'est pas proprietaire de l'appareil.");
        } else {
            text.setText("Mode kiosque desactive.");
        }
    }
}
