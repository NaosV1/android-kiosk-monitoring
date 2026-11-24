# Writes compile-time stubs for the Android APIs used (signatures copied from the Android SDK).
S="$1"; mkdir -p "$S/android/app/admin" "$S/android/content/pm" "$S/android/os" "$S/android/view" "$S/android/widget"
cat > "$S/android/content/Context.java" <<'EOF'
package android.content;
public abstract class Context {
    public Object getSystemService(String name) { return null; }
    public SharedPreferences getSharedPreferences(String name, int mode) { return null; }
    public android.content.pm.PackageManager getPackageManager() { return null; }
    public void startActivity(Intent intent) {}
}
EOF
cat > "$S/android/content/ComponentName.java" <<'EOF'
package android.content;
public final class ComponentName {
    public ComponentName(String pkg, String cls) {}
}
EOF
cat > "$S/android/content/Intent.java" <<'EOF'
package android.content;
public class Intent {
    public Intent() {}
    public Intent(String action) {}
    public Intent setComponent(ComponentName c) { return this; }
    public Intent addFlags(int f) { return this; }
    public Intent addCategory(String c) { return this; }
    public String getStringExtra(String n) { return null; }
    public String getAction() { return null; }
    public String getDataString() { return null; }
}
EOF
cat > "$S/android/content/IntentFilter.java" <<'EOF'
package android.content;
public class IntentFilter {
    public IntentFilter(String action) {}
    public final void addCategory(String c) {}
}
EOF
cat > "$S/android/content/SharedPreferences.java" <<'EOF'
package android.content;
public interface SharedPreferences {
    boolean getBoolean(String k, boolean d);
    Editor edit();
    interface Editor {
        Editor putBoolean(String k, boolean v);
        boolean commit();
    }
}
EOF
cat > "$S/android/content/BroadcastReceiver.java" <<'EOF'
package android.content;
public abstract class BroadcastReceiver {
    public BroadcastReceiver() {}
    public abstract void onReceive(Context c, Intent i);
    public final void setResultData(String d) {}
}
EOF
cat > "$S/android/content/pm/PackageManager.java" <<'EOF'
package android.content.pm;
public abstract class PackageManager {
    public abstract void setComponentEnabledSetting(android.content.ComponentName c, int state, int flags);
}
EOF
cat > "$S/android/app/Activity.java" <<'EOF'
package android.app;
public class Activity extends android.content.Context {
    protected void onCreate(android.os.Bundle b) {}
    protected void onResume() {}
    public void setContentView(android.view.View v) {}
    public void startLockTask() {}
    protected void onNewIntent(android.content.Intent i) {}
    public android.content.Intent getIntent() { return null; }
    public void finish() {}
}
EOF
cat > "$S/android/app/admin/DeviceAdminReceiver.java" <<'EOF'
package android.app.admin;
public class DeviceAdminReceiver extends android.content.BroadcastReceiver {
    public DeviceAdminReceiver() {}
    public void onReceive(android.content.Context c, android.content.Intent i) {}
}
EOF
cat > "$S/android/app/admin/DevicePolicyManager.java" <<'EOF'
package android.app.admin;
import android.content.ComponentName;
import android.content.IntentFilter;
public class DevicePolicyManager {
    public boolean isDeviceOwnerApp(String p) { return false; }
    public void setLockTaskPackages(ComponentName a, String[] p) {}
    public void setLockTaskFeatures(ComponentName a, int f) {}
    public boolean setStatusBarDisabled(ComponentName a, boolean d) { return false; }
    public boolean setKeyguardDisabled(ComponentName a, boolean d) { return false; }
    public void addPersistentPreferredActivity(ComponentName a, IntentFilter f, ComponentName act) {}
    public void clearPackagePersistentPreferredActivities(ComponentName a, String p) {}
    public void clearDeviceOwnerApp(String p) {}
}
EOF
cat > "$S/android/os/Bundle.java" <<'EOF'
package android.os;
public final class Bundle {}
EOF
cat > "$S/android/view/View.java" <<'EOF'
package android.view;
public class View {
    public View(android.content.Context c) {}
    public boolean postDelayed(Runnable r, long delay) { return true; }
    public void setBackgroundColor(int c) {}
}
EOF
cat > "$S/android/widget/TextView.java" <<'EOF'
package android.widget;
public class TextView extends android.view.View {
    public TextView(android.content.Context c) { super(c); }
    public void setTextSize(float s) {}
    public void setGravity(int g) {}
    public void setTextColor(int c) {}
    public final void setText(CharSequence t) {}
}
EOF
