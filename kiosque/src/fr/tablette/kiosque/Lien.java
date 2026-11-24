package fr.tablette.kiosque;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;

/**
 * Recoit les liens https://login.tailscale.com/... (connexion Tailscale) a la place d'un navigateur
 * et les ecrit dans le journal Android (logcat, etiquette System.out, prefixe KIOSQUE_LIEN),
 * pour pouvoir les ouvrir depuis un autre appareil.
 */
public class Lien extends Activity {
    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        noter(getIntent());
        finish();
    }

    @Override
    protected void onNewIntent(Intent i) {
        noter(i);
        finish();
    }

    private void noter(Intent i) {
        String url = i == null ? null : i.getDataString();
        if (url != null && url.startsWith("https://login.tailscale.com/")) {
            System.out.println("KIOSQUE_LIEN " + url);
        }
    }
}
