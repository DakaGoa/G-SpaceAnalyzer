// G-SpaceAnalyzer setup — a self-contained per-user Windows installer.
//
// Built with the in-box .NET Framework compiler (see build-installer.ps1).
// The application executable is embedded as the "Payload" resource, so the
// resulting Setup.exe is a single file that needs no other prerequisites.
//
// Install mode  : installs into a folder of the user's choosing (default
//                 %LOCALAPPDATA%\Programs\G-SpaceAnalyzer), installs
//                 Uninstall.exe, creates a Start menu shortcut, optionally a
//                 desktop shortcut, and registers with Add/Remove Programs.
// Uninstall mode: /uninstall, or running the installed Uninstall.exe. The
//                 target folder is derived from the uninstaller's own location,
//                 so custom install paths are removed correctly.
//
// Silent usage: Setup.exe /S [/D=<folder>] [/DESKTOP]
//               Uninstall.exe /S

using System;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows.Forms;
using Microsoft.Win32;

namespace GSpaceAnalyzerSetup
{
    internal static class Product
    {
        public const string Name = "G-SpaceAnalyzer";
        public static readonly string Version = BuildInfo.Version;
        public const string Publisher = "G-SpaceAnalyzer";
        public const string Tagline = "Disk space analyzer";
        public const string AppExe = "G-SpaceAnalyzer.exe";
        public const string Uninstaller = "Uninstall.exe";
        public const string UninstallKey =
            @"Software\Microsoft\Windows\CurrentVersion\Uninstall\G-SpaceAnalyzer";

        public static string DefaultInstallDir
        {
            get
            {
                string local = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
                return Path.Combine(Path.Combine(local, "Programs"), Name);
            }
        }

        public static string StartMenuShortcut
        {
            get
            {
                string programs = Environment.GetFolderPath(Environment.SpecialFolder.Programs);
                return Path.Combine(programs, Name + ".lnk");
            }
        }

        public static string DesktopShortcut
        {
            get
            {
                string desktop = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
                return Path.Combine(desktop, Name + ".lnk");
            }
        }

        public static Color DarkBackground { get { return Color.FromArgb(20, 22, 27); } }
        public static Color DarkSurface { get { return Color.FromArgb(30, 33, 40); } }
        public static Color TextColor { get { return Color.FromArgb(236, 239, 244); } }
        public static Color MutedColor { get { return Color.FromArgb(150, 157, 170); } }
        public static Color AccentColor { get { return Color.FromArgb(91, 134, 255); } }
        public static Color BorderColor { get { return Color.FromArgb(58, 63, 76); } }
    }

    internal static class Program
    {
        [STAThread]
        private static int Main(string[] args)
        {
            bool silent = HasSwitch(args, "/S") || HasSwitch(args, "--silent");
            bool uninstall = HasSwitch(args, "/uninstall") ||
                             string.Equals(Path.GetFileName(Application.ExecutablePath),
                                           Product.Uninstaller, StringComparison.OrdinalIgnoreCase);

            if (silent)
            {
                return RunSilent(uninstall, args);
            }

            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            using (SetupForm form = new SetupForm(uninstall))
            {
                Application.Run(form);
                return form.ExitCode;
            }
        }

        private static int RunSilent(bool uninstall, string[] args)
        {
            StringBuilder log = new StringBuilder();
            log.AppendLine(Product.Name + " setup " + Product.Version +
                           (uninstall ? " (uninstall)" : " (install)"));
            try
            {
                Installer installer = new Installer();
                Action<string> report = delegate(string message) { log.AppendLine(message); };

                if (uninstall)
                {
                    string directory = Installer.InstalledDirectory();
                    if (string.IsNullOrEmpty(directory))
                    {
                        log.AppendLine("Not installed; nothing to do.");
                        return 0;
                    }
                    installer.Uninstall(report, directory);
                }
                else
                {
                    string directory = ReadDirectoryArg(args);
                    if (string.IsNullOrEmpty(directory))
                    {
                        directory = Product.DefaultInstallDir;
                    }
                    directory = Installer.NormalizeDirectory(directory);
                    bool desktop = HasSwitch(args, "/DESKTOP") || HasSwitch(args, "/desktop");
                    installer.Install(report, directory, desktop);
                }

                log.AppendLine("OK");
                return 0;
            }
            catch (Exception error)
            {
                log.AppendLine("FAILED: " + error.Message);
                return 1;
            }
            finally
            {
                try
                {
                    string logPath = Path.Combine(Path.GetTempPath(), "GSpaceAnalyzerSetup.log");
                    File.WriteAllText(logPath, log.ToString());
                }
                catch
                {
                    // Logging is best effort.
                }
            }
        }

        private static string ReadDirectoryArg(string[] args)
        {
            if (args == null)
            {
                return null;
            }
            for (int i = 0; i < args.Length; i++)
            {
                string argument = args[i];
                if (argument.StartsWith("/D=", StringComparison.OrdinalIgnoreCase) ||
                    argument.StartsWith("/DIR=", StringComparison.OrdinalIgnoreCase))
                {
                    return argument.Substring(argument.IndexOf('=') + 1).Trim().Trim('"');
                }
            }
            return null;
        }

        private static bool HasSwitch(string[] args, string name)
        {
            if (args == null)
            {
                return false;
            }
            for (int i = 0; i < args.Length; i++)
            {
                if (string.Equals(args[i], name, StringComparison.OrdinalIgnoreCase))
                {
                    return true;
                }
            }
            return false;
        }
    }

    internal sealed class Installer
    {
        public void Install(Action<string> report, string directory, bool desktopShortcut)
        {
            CloseRunningApp();
            report("Reading application payload");
            byte[] payload = ReadResource("Payload");
            if (payload == null || payload.Length == 0)
            {
                throw new InvalidOperationException("The installer payload is missing.");
            }

            report("Creating " + directory);
            Directory.CreateDirectory(directory);

            string appPath = Path.Combine(directory, Product.AppExe);
            string uninstallerPath = Path.Combine(directory, Product.Uninstaller);

            report("Installing G-SpaceAnalyzer.exe");
            File.WriteAllBytes(appPath, payload);

            report("Installing the uninstaller");
            File.Copy(Application.ExecutablePath, uninstallerPath, true);

            report("Creating the Start menu shortcut");
            CreateShortcut(Product.StartMenuShortcut, appPath, directory,
                           Product.Name + " — " + Product.Tagline);

            if (desktopShortcut)
            {
                report("Creating the desktop shortcut");
                CreateShortcut(Product.DesktopShortcut, appPath, directory,
                               Product.Name + " — " + Product.Tagline);
            }
            else
            {
                DeleteFile(Product.DesktopShortcut);
            }

            report("Registering with Add or Remove Programs");
            WriteUninstallRegistry(directory, appPath, uninstallerPath, payload.Length);
        }

        public void Uninstall(Action<string> report, string directory)
        {
            report("Closing G-SpaceAnalyzer");
            CloseRunningApp();

            report("Removing shortcuts");
            DeleteFile(Product.StartMenuShortcut);
            DeleteFile(Product.DesktopShortcut);

            report("Removing registry entries");
            try
            {
                Registry.CurrentUser.DeleteSubKeyTree(Product.UninstallKey, false);
            }
            catch (Exception)
            {
                // A missing key is not an error.
            }

            report("Removing application files");
            DeleteFile(Path.Combine(directory, Product.AppExe));

            report("Cleaning up");
            ScheduleDirectoryRemoval(directory);
        }

        /// <summary>The folder holding the running uninstaller, else the registered location.</summary>
        public static string InstalledDirectory()
        {
            string ownFolder = Path.GetDirectoryName(Application.ExecutablePath);
            if (!string.IsNullOrEmpty(ownFolder) &&
                File.Exists(Path.Combine(ownFolder, Product.AppExe)))
            {
                return ownFolder;
            }

            using (RegistryKey key = Registry.CurrentUser.OpenSubKey(Product.UninstallKey))
            {
                if (key != null)
                {
                    string location = key.GetValue("InstallLocation") as string;
                    if (!string.IsNullOrEmpty(location) && Directory.Exists(location))
                    {
                        return location;
                    }
                }
            }
            return null;
        }

        /// <summary>Expands and trims a user supplied folder, rejecting nonsense.</summary>
        public static string NormalizeDirectory(string directory)
        {
            if (string.IsNullOrEmpty(directory) || directory.Trim().Length == 0)
            {
                throw new InvalidOperationException("Choose an install folder.");
            }
            string expanded = Environment.ExpandEnvironmentVariables(directory.Trim().Trim('"'));
            string full;
            try
            {
                full = Path.GetFullPath(expanded);
            }
            catch (Exception)
            {
                throw new InvalidOperationException("That folder path is not valid.");
            }
            full = full.TrimEnd('\\');
            if (full.Length == 2 && full[1] == ':')
            {
                throw new InvalidOperationException("Choose a folder, not a whole drive.");
            }
            string windows = Environment.GetFolderPath(Environment.SpecialFolder.Windows);
            if (full.StartsWith(windows, StringComparison.OrdinalIgnoreCase))
            {
                throw new InvalidOperationException("Windows system folders cannot be used.");
            }
            return full;
        }

        private static byte[] ReadResource(string name)
        {
            using (Stream stream = Assembly.GetExecutingAssembly().GetManifestResourceStream(name))
            {
                if (stream == null)
                {
                    return null;
                }
                int length = (int)stream.Length;
                byte[] buffer = new byte[length];
                int read = 0;
                while (read < length)
                {
                    int chunk = stream.Read(buffer, read, length - read);
                    if (chunk <= 0)
                    {
                        break;
                    }
                    read += chunk;
                }
                return buffer;
            }
        }

        private static void WriteUninstallRegistry(string directory, string appPath,
                                                   string uninstallerPath, int payloadSize)
        {
            using (RegistryKey key = Registry.CurrentUser.CreateSubKey(Product.UninstallKey))
            {
                if (key == null)
                {
                    throw new InvalidOperationException("Could not create the uninstall registry key.");
                }
                key.SetValue("DisplayName", Product.Name);
                key.SetValue("DisplayVersion", Product.Version);
                key.SetValue("Publisher", Product.Publisher);
                key.SetValue("DisplayIcon", appPath + ",0");
                key.SetValue("InstallLocation", directory);
                key.SetValue("UninstallString", "\"" + uninstallerPath + "\"");
                key.SetValue("QuietUninstallString", "\"" + uninstallerPath + "\" /S");
                key.SetValue("InstallDate", DateTime.Now.ToString("yyyyMMdd"));
                key.SetValue("EstimatedSize", Math.Max(1, payloadSize / 1024), RegistryValueKind.DWord);
                key.SetValue("NoModify", 1, RegistryValueKind.DWord);
                key.SetValue("NoRepair", 1, RegistryValueKind.DWord);
            }
        }

        private static void DeleteFile(string path)
        {
            try
            {
                if (File.Exists(path))
                {
                    File.Delete(path);
                }
            }
            catch (Exception)
            {
                // Locked files are cleaned up by the deferred directory removal.
            }
        }

        private static void CloseRunningApp()
        {
            Process[] running = Process.GetProcessesByName("G-SpaceAnalyzer");
            if (running.Length == 0)
            {
                return;
            }
            for (int i = 0; i < running.Length; i++)
            {
                try
                {
                    running[i].CloseMainWindow();
                }
                catch (Exception)
                {
                    // Ignore processes that cannot be signalled.
                }
            }
            for (int i = 0; i < running.Length; i++)
            {
                try
                {
                    if (!running[i].WaitForExit(3000))
                    {
                        running[i].Kill();
                        running[i].WaitForExit(2000);
                    }
                }
                catch (Exception)
                {
                    // Ignore.
                }
                finally
                {
                    running[i].Dispose();
                }
            }
        }

        // The uninstaller cannot delete the folder it is running from, so hand
        // the removal to a detached shell once this process has exited.
        private static void ScheduleDirectoryRemoval(string directory)
        {
            try
            {
                string arguments = "/c ping 127.0.0.1 -n 3 > nul & rmdir /s /q \"" + directory + "\"";
                ProcessStartInfo info = new ProcessStartInfo("cmd.exe", arguments);
                info.UseShellExecute = false;
                info.CreateNoWindow = true;
                info.WorkingDirectory = Path.GetTempPath();
                Process.Start(info);
            }
            catch (Exception)
            {
                // Best effort.
            }
        }

        // --- Shell link -----------------------------------------------------

        private static void CreateShortcut(string shortcutPath, string target, string workingDir,
                                           string description)
        {
            string parent = Path.GetDirectoryName(shortcutPath);
            if (!string.IsNullOrEmpty(parent))
            {
                Directory.CreateDirectory(parent);
            }

            ShellLink link = new ShellLink();
            ((IShellLinkW)link).SetPath(target);
            ((IShellLinkW)link).SetWorkingDirectory(workingDir);
            ((IShellLinkW)link).SetDescription(description);
            ((IShellLinkW)link).SetIconLocation(target, 0);
            ((IPersistFile)link).Save(shortcutPath, true);
        }

        [ComImport]
        [Guid("00021401-0000-0000-C000-000000000046")]
        private class ShellLink
        {
        }

        [ComImport]
        [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        [Guid("000214F9-0000-0000-C000-000000000046")]
        private interface IShellLinkW
        {
            void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszFile,
                         int cchMaxPath, IntPtr pfd, uint fFlags);
            void GetIDList(out IntPtr ppidl);
            void SetIDList(IntPtr pidl);
            void GetDescription([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszName,
                                int cchMaxName);
            void SetDescription([MarshalAs(UnmanagedType.LPWStr)] string pszName);
            void GetWorkingDirectory([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszDir,
                                     int cchMaxPath);
            void SetWorkingDirectory([MarshalAs(UnmanagedType.LPWStr)] string pszDir);
            void GetArguments([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszArgs,
                              int cchMaxPath);
            void SetArguments([MarshalAs(UnmanagedType.LPWStr)] string pszArgs);
            void GetHotkey(out short pwHotkey);
            void SetHotkey(short wHotkey);
            void GetShowCmd(out int piShowCmd);
            void SetShowCmd(int iShowCmd);
            void GetIconLocation([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder pszIconPath,
                                 int cchIconPath, out int piIcon);
            void SetIconLocation([MarshalAs(UnmanagedType.LPWStr)] string pszIconPath, int iIcon);
            void SetRelativePath([MarshalAs(UnmanagedType.LPWStr)] string pszPathRel,
                                 uint dwReserved);
            void Resolve(IntPtr hwnd, uint fFlags);
            void SetPath([MarshalAs(UnmanagedType.LPWStr)] string pszFile);
        }

        [ComImport]
        [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        [Guid("0000010b-0000-0000-C000-000000000046")]
        private interface IPersistFile
        {
            void GetClassID(out Guid pClassID);
            [PreserveSig]
            int IsDirty();
            void Load([MarshalAs(UnmanagedType.LPWStr)] string pszFileName, uint dwMode);
            void Save([MarshalAs(UnmanagedType.LPWStr)] string pszFileName,
                      [MarshalAs(UnmanagedType.Bool)] bool fRemember);
            void SaveCompleted([MarshalAs(UnmanagedType.LPWStr)] string pszFileName);
            void GetCurFile([MarshalAs(UnmanagedType.LPWStr)] out string ppszFileName);
        }
    }

    internal sealed class SetupForm : Form
    {
        private readonly bool uninstallMode;
        private readonly string installedDirectory;
        private readonly Label headingLabel;
        private readonly Label statusLabel;
        private readonly Label pathLabel;
        private readonly TextBox pathBox;
        private readonly Button browseButton;
        private readonly ToggleCheck desktopCheck;
        private readonly Button primaryButton;
        private readonly Button cancelButton;
        private readonly ToggleCheck launchCheck;
        private readonly Panel progressTrack;
        private readonly Panel progressFill;
        private int progressSteps;
        private bool finished;

        public int ExitCode { get; private set; }

        public SetupForm(bool uninstallMode)
        {
            this.uninstallMode = uninstallMode;
            ExitCode = 1;
            progressSteps = 0;
            installedDirectory = uninstallMode ? Installer.InstalledDirectory() : null;

            Text = uninstallMode ? "Uninstall G-SpaceAnalyzer" : "G-SpaceAnalyzer Setup";
            ClientSize = new Size(486, 352);
            FormBorderStyle = FormBorderStyle.FixedDialog;
            StartPosition = FormStartPosition.CenterScreen;
            MaximizeBox = false;
            MinimizeBox = false;
            ShowIcon = true;
            BackColor = Product.DarkBackground;
            ForeColor = Product.TextColor;
            Font = new Font("Segoe UI", 9F);
            DoubleBuffered = true;

            Icon formIcon = LoadIcon();
            if (formIcon != null)
            {
                Icon = formIcon;
            }

            PictureBox iconBox = new PictureBox();
            iconBox.Bounds = new Rectangle(28, 24, 46, 46);
            iconBox.SizeMode = PictureBoxSizeMode.Zoom;
            iconBox.BackColor = Color.Transparent;
            iconBox.Image = LoadIconImage();
            Controls.Add(iconBox);

            Label titleLabel = new Label();
            titleLabel.Text = Product.Name;
            titleLabel.Font = new Font("Segoe UI", 17F, FontStyle.Bold);
            titleLabel.ForeColor = Product.TextColor;
            titleLabel.AutoSize = true;
            titleLabel.Location = new Point(90, 22);
            titleLabel.BackColor = Color.Transparent;
            Controls.Add(titleLabel);

            Label subtitleLabel = new Label();
            subtitleLabel.Text = uninstallMode
                ? "Remove G-SpaceAnalyzer from this computer"
                : Product.Tagline + "  ·  Version " + Product.Version;
            subtitleLabel.Font = new Font("Segoe UI", 9F);
            subtitleLabel.ForeColor = Product.MutedColor;
            subtitleLabel.AutoSize = true;
            subtitleLabel.Location = new Point(92, 52);
            subtitleLabel.BackColor = Color.Transparent;
            Controls.Add(subtitleLabel);

            Panel divider = new Panel();
            divider.Bounds = new Rectangle(28, 92, 430, 1);
            divider.BackColor = Product.BorderColor;
            Controls.Add(divider);

            pathLabel = new Label();
            pathLabel.Text = "Install folder";
            pathLabel.ForeColor = Product.MutedColor;
            pathLabel.Bounds = new Rectangle(28, 104, 200, 18);
            pathLabel.BackColor = Color.Transparent;
            Controls.Add(pathLabel);

            pathBox = new TextBox();
            pathBox.Text = Product.DefaultInstallDir;
            pathBox.Bounds = new Rectangle(28, 124, 346, 26);
            pathBox.BorderStyle = BorderStyle.FixedSingle;
            pathBox.BackColor = Product.DarkSurface;
            pathBox.ForeColor = Product.TextColor;
            pathBox.Font = new Font("Segoe UI", 9F);
            Controls.Add(pathBox);

            browseButton = new Button();
            browseButton.Text = "Browse…";
            browseButton.Bounds = new Rectangle(382, 123, 76, 28);
            StyleSecondary(browseButton);
            browseButton.Click += OnBrowseClick;
            Controls.Add(browseButton);

            desktopCheck = new ToggleCheck();
            desktopCheck.Text = "Create a desktop shortcut";
            desktopCheck.Bounds = new Rectangle(26, 158, 300, 26);
            desktopCheck.Checked = false;
            Controls.Add(desktopCheck);

            headingLabel = new Label();
            headingLabel.Font = new Font("Segoe UI", 11F, FontStyle.Bold);
            headingLabel.ForeColor = Product.TextColor;
            headingLabel.AutoSize = true;
            headingLabel.BackColor = Color.Transparent;
            Controls.Add(headingLabel);

            statusLabel = new Label();
            statusLabel.ForeColor = Product.MutedColor;
            statusLabel.Bounds = new Rectangle(28, 224, 430, 54);
            statusLabel.BackColor = Color.Transparent;
            Controls.Add(statusLabel);

            progressTrack = new Panel();
            progressTrack.BackColor = Product.DarkSurface;
            progressTrack.Visible = false;
            Controls.Add(progressTrack);

            progressFill = new Panel();
            progressFill.Bounds = new Rectangle(0, 0, 0, 4);
            progressFill.BackColor = Product.AccentColor;
            progressTrack.Controls.Add(progressFill);

            launchCheck = new ToggleCheck();
            launchCheck.Text = "Open G-SpaceAnalyzer when finished";
            launchCheck.Bounds = new Rectangle(26, 288, 240, 26);
            launchCheck.Visible = false;
            launchCheck.Checked = true;
            Controls.Add(launchCheck);

            primaryButton = new Button();
            primaryButton.Text = uninstallMode ? "Remove" : "Install";
            primaryButton.Bounds = new Rectangle(274, 282, 184, 38);
            primaryButton.FlatStyle = FlatStyle.Flat;
            primaryButton.FlatAppearance.BorderSize = 0;
            primaryButton.BackColor = uninstallMode ? Color.FromArgb(198, 63, 54) : Product.AccentColor;
            primaryButton.ForeColor = Color.White;
            primaryButton.Font = new Font("Segoe UI", 9.5F, FontStyle.Bold);
            primaryButton.Cursor = Cursors.Hand;
            primaryButton.Click += OnPrimaryClick;
            Controls.Add(primaryButton);

            cancelButton = new Button();
            cancelButton.Text = "Cancel";
            cancelButton.Bounds = new Rectangle(390, 282, 68, 38);
            StyleSecondary(cancelButton);
            cancelButton.Click += delegate { Close(); };
            Controls.Add(cancelButton);

            if (uninstallMode)
            {
                // The folder is implied by the installed uninstaller, so show it
                // as text rather than an editable field.
                pathLabel.Visible = false;
                pathBox.Visible = false;
                browseButton.Visible = false;
                desktopCheck.Visible = false;
                headingLabel.Location = new Point(28, 108);
                headingLabel.Text = string.IsNullOrEmpty(installedDirectory)
                    ? "G-SpaceAnalyzer is not installed"
                    : "Ready to remove";
                statusLabel.Location = new Point(28, 136);
                statusLabel.Text = string.IsNullOrEmpty(installedDirectory)
                    ? "Nothing was found to remove for this user."
                    : "This removes the application and its shortcuts from:\r\n" + installedDirectory +
                      "\r\nYour settings in %APPDATA%\\G-SpaceAnalyzer are kept.";
                progressTrack.Bounds = new Rectangle(28, 262, 424, 4);

                if (string.IsNullOrEmpty(installedDirectory))
                {
                    primaryButton.Text = "Close";
                    primaryButton.BackColor = Product.AccentColor;
                    finished = true;
                    ExitCode = 0;
                }
            }
            else
            {
                headingLabel.Location = new Point(28, 196);
                headingLabel.Text = "Ready to install";
                statusLabel.Location = new Point(28, 224);
                statusLabel.Text =
                    "G-SpaceAnalyzer installs for the current user only — no administrator rights needed.";
                progressTrack.Bounds = new Rectangle(28, 268, 424, 4);
            }

            AcceptButton = primaryButton;
            CancelButton = cancelButton;
        }

        private static void StyleSecondary(Button button)
        {
            button.FlatStyle = FlatStyle.Flat;
            button.FlatAppearance.BorderSize = 1;
            button.FlatAppearance.BorderColor = Product.BorderColor;
            button.BackColor = Product.DarkBackground;
            button.ForeColor = Product.MutedColor;
            button.Font = new Font("Segoe UI", 9F);
            button.Cursor = Cursors.Hand;
        }

        // The stock CheckBox draws its tick with a system colour that is
        // invisible on the dark form (so ticking it looked like nothing
        // happened), so both options are drawn and hit-tested here instead.
        private sealed class ToggleCheck : Control
        {
            private bool isChecked;

            public ToggleCheck()
            {
                SetStyle(ControlStyles.UserPaint | ControlStyles.AllPaintingInWmPaint |
                         ControlStyles.OptimizedDoubleBuffer | ControlStyles.ResizeRedraw, true);
                BackColor = Product.DarkBackground;
                ForeColor = Product.TextColor;
                Font = new Font("Segoe UI", 9F);
                Cursor = Cursors.Hand;
                TabStop = true;
            }

            public bool Checked
            {
                get { return isChecked; }
                set
                {
                    if (isChecked == value)
                    {
                        return;
                    }

                    isChecked = value;
                    Invalidate();
                }
            }

            protected override void OnClick(EventArgs e)
            {
                if (Enabled)
                {
                    isChecked = !isChecked;
                    Invalidate();
                }

                base.OnClick(e);
            }

            protected override void OnKeyDown(KeyEventArgs e)
            {
                if (Enabled && e.KeyCode == Keys.Space)
                {
                    isChecked = !isChecked;
                    Invalidate();
                    e.Handled = true;
                }

                base.OnKeyDown(e);
            }

            protected override void OnPaint(PaintEventArgs e)
            {
                Graphics g = e.Graphics;
                g.Clear(BackColor);
                g.SmoothingMode = SmoothingMode.AntiAlias;

                int box = Math.Max(14, Font.Height - 2);
                int top = (Height - box) / 2;
                Rectangle boxRect = new Rectangle(1, top, box - 1, box - 1);
                Color edge = isChecked ? Product.AccentColor : Product.BorderColor;

                using (GraphicsPath path = RoundedRectangle(boxRect, 4))
                using (SolidBrush fill = new SolidBrush(isChecked ? Product.AccentColor : Product.DarkSurface))
                using (Pen border = new Pen(edge, 1.4f))
                {
                    g.FillPath(fill, path);
                    g.DrawPath(border, path);
                }

                if (isChecked)
                {
                    float unit = box / 15f;
                    using (Pen tick = new Pen(Color.White, 1.9f * unit))
                    {
                        tick.StartCap = LineCap.Round;
                        tick.EndCap = LineCap.Round;
                        tick.LineJoin = LineJoin.Round;
                        g.DrawLines(tick, new PointF[]
                        {
                            new PointF(4.2f * unit, top + 7.6f * unit),
                            new PointF(6.6f * unit, top + 10.2f * unit),
                            new PointF(11.2f * unit, top + 4.6f * unit),
                        });
                    }
                }

                Color textColor = Enabled ? ForeColor : Product.MutedColor;
                int textLeft = box + 9;
                Size textSize = TextRenderer.MeasureText(Text, Font, Size.Empty, TextFormatFlags.NoPadding);
                TextRenderer.DrawText(g, Text, Font,
                    new Point(textLeft, (Height - textSize.Height) / 2),
                    textColor, TextFormatFlags.NoPadding);
            }

            private static GraphicsPath RoundedRectangle(Rectangle bounds, int radius)
            {
                int diameter = radius * 2;
                GraphicsPath path = new GraphicsPath();
                path.AddArc(bounds.X, bounds.Y, diameter, diameter, 180, 90);
                path.AddArc(bounds.Right - diameter, bounds.Y, diameter, diameter, 270, 90);
                path.AddArc(bounds.Right - diameter, bounds.Bottom - diameter, diameter, diameter, 0, 90);
                path.AddArc(bounds.X, bounds.Bottom - diameter, diameter, diameter, 90, 90);
                path.CloseFigure();
                return path;
            }
        }

        private static Icon LoadIcon()
        {
            try
            {
                using (Stream stream = Assembly.GetExecutingAssembly()
                           .GetManifestResourceStream("AppIcon"))
                {
                    if (stream != null)
                    {
                        return new Icon(stream);
                    }
                }
            }
            catch (Exception)
            {
                // Fall back to the default window icon.
            }
            return null;
        }

        // GDI+'s Icon.ToBitmap() garbles icon frames that are stored as PNG
        // (all but the smallest sizes in our .ico), so pull the frame out of the
        // resource ourselves and let the PNG decoder handle it. The shell-drawn
        // window icon is unaffected because that path uses the icon handle.
        private static Image LoadIconImage()
        {
            try
            {
                using (Stream stream = Assembly.GetExecutingAssembly()
                           .GetManifestResourceStream("AppIcon"))
                {
                    if (stream != null)
                    {
                        using (MemoryStream buffer = new MemoryStream())
                        {
                            stream.CopyTo(buffer);
                            Image image = DecodeBestFrame(buffer.ToArray(), 128);
                            if (image != null)
                            {
                                return image;
                            }
                        }
                    }
                }
            }
            catch (Exception)
            {
                // Fall back to the shell icon below.
            }

            try
            {
                Icon shellIcon = Icon.ExtractAssociatedIcon(Application.ExecutablePath);
                if (shellIcon != null)
                {
                    using (shellIcon)
                    {
                        return shellIcon.ToBitmap();
                    }
                }
            }
            catch (Exception)
            {
                // No icon at all then; the header simply has no picture.
            }
            return null;
        }

        // Picks the largest PNG-compressed frame up to maxSize and decodes it.
        private static Image DecodeBestFrame(byte[] data, int maxSize)
        {
            if (data.Length < 22)
            {
                return null;
            }

            int count = BitConverter.ToUInt16(data, 4);
            int bestOffset = -1;
            int bestBytes = 0;
            int bestWidth = -1;
            for (int i = 0; i < count; i++)
            {
                int entry = 6 + i * 16;
                if (entry + 16 > data.Length)
                {
                    break;
                }

                int width = data[entry] == 0 ? 256 : data[entry];
                int bytes = BitConverter.ToInt32(data, entry + 8);
                int offset = BitConverter.ToInt32(data, entry + 12);
                if (width > maxSize || bytes <= 0 || offset <= 0 || offset + bytes > data.Length)
                {
                    continue;
                }

                // 0x89 is the first byte of the PNG signature.
                if (data[offset] != 0x89)
                {
                    continue;
                }

                if (width > bestWidth)
                {
                    bestWidth = width;
                    bestOffset = offset;
                    bestBytes = bytes;
                }
            }

            if (bestOffset < 0)
            {
                return null;
            }

            using (MemoryStream png = new MemoryStream(data, bestOffset, bestBytes))
            using (Image decoded = Image.FromStream(png))
            {
                // Copy so the bitmap does not depend on the stream staying open.
                return new Bitmap(decoded);
            }
        }

        private void OnBrowseClick(object sender, EventArgs e)
        {
            using (FolderBrowserDialog dialog = new FolderBrowserDialog())
            {
                dialog.Description = "Choose the folder to install G-SpaceAnalyzer into";
                dialog.ShowNewFolderButton = true;
                string current = pathBox.Text.Trim();
                if (current.Length > 0 && Directory.Exists(current))
                {
                    dialog.SelectedPath = current;
                }
                else
                {
                    string parent = Path.GetDirectoryName(current);
                    if (!string.IsNullOrEmpty(parent) && Directory.Exists(parent))
                    {
                        dialog.SelectedPath = parent;
                    }
                }
                if (dialog.ShowDialog(this) == DialogResult.OK)
                {
                    pathBox.Text = dialog.SelectedPath;
                }
            }
        }

        private void OnPrimaryClick(object sender, EventArgs e)
        {
            if (finished)
            {
                if (!uninstallMode && launchCheck.Checked && launchCheck.Visible)
                {
                    try
                    {
                        Process.Start(Path.Combine(pathBox.Text.Trim(), Product.AppExe));
                    }
                    catch (Exception)
                    {
                        // Starting the app is optional.
                    }
                }
                Close();
                return;
            }
            RunWork();
        }

        private void RunWork()
        {
            string directory = uninstallMode ? installedDirectory : null;
            if (!uninstallMode)
            {
                try
                {
                    directory = Installer.NormalizeDirectory(pathBox.Text);
                }
                catch (Exception error)
                {
                    MessageBox.Show(this, error.Message, "G-SpaceAnalyzer Setup",
                                    MessageBoxButtons.OK, MessageBoxIcon.Warning);
                    pathBox.Focus();
                    pathBox.SelectAll();
                    return;
                }
            }

            pathBox.Text = directory;
            pathBox.Enabled = false;
            browseButton.Enabled = false;
            desktopCheck.Enabled = false;
            primaryButton.Enabled = false;
            cancelButton.Enabled = false;
            progressTrack.Visible = true;
            finished = false;

            Installer installer = new Installer();
            Action<string> report = delegate(string message)
            {
                progressSteps++;
                headingLabel.Text = message;
                progressFill.Width = Math.Min(424, progressSteps * 68);
                Refresh();
                Application.DoEvents();
            };

            try
            {
                if (uninstallMode)
                {
                    installer.Uninstall(report, directory);
                    progressFill.Width = 424;
                    headingLabel.Text = "G-SpaceAnalyzer has been removed";
                    statusLabel.Text = "The application and its shortcuts are gone. "
                                     + "Your settings in %APPDATA%\\G-SpaceAnalyzer were kept.";
                }
                else
                {
                    installer.Install(report, directory, desktopCheck.Checked);
                    progressFill.Width = 424;
                    headingLabel.Text = "G-SpaceAnalyzer is installed";
                    statusLabel.Text = "Installed in:\r\n" + directory
                                     + "\r\nA shortcut was added to your Start menu"
                                     + (desktopCheck.Checked ? " and your desktop." : ".");
                    launchCheck.Visible = true;
                }
                finished = true;
                primaryButton.Text = "Close";
                primaryButton.Enabled = true;
                cancelButton.Visible = false;
                ExitCode = 0;
            }
            catch (Exception error)
            {
                headingLabel.Text = "Setup could not finish";
                statusLabel.Text = error.Message;
                pathBox.Enabled = !uninstallMode;
                browseButton.Enabled = !uninstallMode;
                desktopCheck.Enabled = !uninstallMode;
                primaryButton.Text = "Close";
                primaryButton.Enabled = true;
                cancelButton.Visible = false;
                ExitCode = 1;
            }
        }
    }
}
