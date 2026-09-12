using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Text.RegularExpressions;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using NHotkey;
using NHotkey.Wpf;

namespace ZaplRecorder
{
    public partial class MainWindow : Window
    {
        private Process? _ffmpegProcess;
        private DispatcherTimer _monitorTimer = null!;
        private DispatcherTimer _recordTimer = null!;
        private DateTime _startTime;
        private DateTime _launchTime;
        private bool _isRecording;
        private string _ffmpegPath = null!;

        private PerformanceCounter? _cpuCounter;

        private AppSettings _settings = null!;

        // Очередь попыток запуска на текущую запись: (кодировщик, звук).
        // Если попытка падает в первые ~2.5 сек, автоматически пробуем следующую.
        private List<(string Encoder, bool Audio)> _attemptQueue = new();
        private int _attemptIndex;
        private string _currentOutputFile = "";
        private string _currentEncoder = "";

        private static readonly Regex StatsRegex =
            new Regex(@"fps=\s*([\d\.]+).*?bitrate=\s*([\d\.]+)kbits/s", RegexOptions.Compiled);

        public MainWindow()
        {
            InitializeComponent();
            headerBar.MouseLeftButtonDown += HeaderBar_MouseLeftButtonDown;
            this.Loaded += (sender, args) => InitializeApp();
        }

        // Двигаем окно только за область заголовка и только если клик не по кнопке
        // (иначе окно "тащится" при клике по любому элементу управления, включая
        // будущие поля настроек — TextBox, ComboBox и т.д.)
        private void HeaderBar_MouseLeftButtonDown(object sender, MouseButtonEventArgs e)
        {
            if (e.OriginalSource is DependencyObject d && FindAncestor<ButtonBase>(d) != null)
                return;

            if (e.LeftButton == MouseButtonState.Pressed)
                DragMove();
        }

        private static T? FindAncestor<T>(DependencyObject current) where T : DependencyObject
        {
            while (current != null)
            {
                if (current is T match) return match;
                current = VisualTreeHelper.GetParent(current);
            }
            return null;
        }

        private void InitializeApp()
        {
            _ffmpegPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "ffmpeg.exe");
            if (!File.Exists(_ffmpegPath)) _ffmpegPath = "ffmpeg.exe";

            _settings = AppSettings.Load();
            ApplySettingsToUi();

            try
            {
                _cpuCounter = new PerformanceCounter("Processor", "% Processor Time", "_Total");
                _cpuCounter.NextValue();
            }
            catch { }

            _monitorTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
            _monitorTimer.Tick += MonitorResources;
            _monitorTimer.Start();

            _recordTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
            _recordTimer.Tick += (s, e) =>
            {
                var elapsed = DateTime.Now - _startTime;
                lblTimer.Text = elapsed.ToString(@"hh\:mm\:ss");
            };

            try
            {
                HotkeyManager.Current.AddOrReplace("ToggleRec", Key.F9, ModifierKeys.Control, (s, e) => ToggleRecording());
            }
            catch { }
        }

        private void ApplySettingsToUi()
        {
            foreach (ComboBoxItem it in comboFps.Items)
                if (it.Content?.ToString() == _settings.Fps.ToString()) { comboFps.SelectedItem = it; break; }
            if (comboFps.SelectedItem == null) comboFps.SelectedIndex = 2; // 30 по умолчанию

            foreach (ComboBoxItem it in comboEncoder.Items)
                if ((string)it.Tag == _settings.Encoder) { comboEncoder.SelectedItem = it; break; }
            if (comboEncoder.SelectedItem == null) comboEncoder.SelectedIndex = 0;

            foreach (ComboBoxItem it in comboScale.Items)
                if ((string)it.Tag == _settings.ScalePercent.ToString()) { comboScale.SelectedItem = it; break; }
            if (comboScale.SelectedItem == null) comboScale.SelectedIndex = 0;

            foreach (ComboBoxItem it in comboBitrate.Items)
                if ((string)it.Tag == _settings.BitrateKbps.ToString()) { comboBitrate.SelectedItem = it; break; }
            if (comboBitrate.SelectedItem == null) comboBitrate.SelectedIndex = 2;

            chkAudio.IsChecked = _settings.RecordAudio;
            chkStealthSettings.IsChecked = _settings.StealthMode;

            txtOutputFolder.Text = string.IsNullOrWhiteSpace(_settings.OutputFolder)
                ? Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Videos") + "  (по умолчанию)"
                : _settings.OutputFolder;
        }

        private void MonitorResources(object? sender, EventArgs e)
        {
            if (_cpuCounter != null)
            {
                try
                {
                    float cpuUsage = _cpuCounter.NextValue();
                    lblCpuLoad.Text = $"{(int)cpuUsage}%";
                    pbCpu.Value = cpuUsage;

                    if (cpuUsage > 90) pbCpu.Foreground = Brushes.Red;
                    else pbCpu.Foreground = new SolidColorBrush(Color.FromRgb(59, 130, 246));
                }
                catch { }
            }
        }

        // ===================== Вкладки =====================

        private void btnTabRecord_Click(object sender, RoutedEventArgs e)
        {
            btnTabRecord.IsEnabled = false;
            btnTabSettings.IsEnabled = true;
            gridRecordView.Visibility = Visibility.Visible;
            gridSettingsView.Visibility = Visibility.Collapsed;
            footerRecord.Visibility = Visibility.Visible;
            lblTimer.Visibility = Visibility.Visible;
            UpdateUIState();
        }

        private void btnTabSettings_Click(object sender, RoutedEventArgs e)
        {
            btnTabSettings.IsEnabled = false;
            btnTabRecord.IsEnabled = true;
            gridRecordView.Visibility = Visibility.Collapsed;
            gridSettingsView.Visibility = Visibility.Visible;
            footerRecord.Visibility = Visibility.Collapsed;
            lblHeaderTitle.Text = "Настройки";
            lblTimer.Visibility = Visibility.Collapsed;
        }

        // ===================== Обработчики настроек =====================

        private void comboFps_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (_settings == null) return;
            if (comboFps.SelectedItem is ComboBoxItem item && int.TryParse(item.Content?.ToString(), out int fps))
            {
                _settings.Fps = fps;
                _settings.Save();
            }
        }

        private void comboEncoder_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (_settings == null) return;
            if (comboEncoder.SelectedItem is ComboBoxItem item && item.Tag is string tag)
            {
                _settings.Encoder = tag;
                _settings.Save();
            }
        }

        private void comboScale_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (_settings == null) return;
            if (comboScale.SelectedItem is ComboBoxItem item && item.Tag is string tag && int.TryParse(tag, out int val))
            {
                _settings.ScalePercent = val;
                _settings.Save();
            }
        }

        private void comboBitrate_SelectionChanged(object sender, SelectionChangedEventArgs e)
        {
            if (_settings == null) return;
            if (comboBitrate.SelectedItem is ComboBoxItem item && item.Tag is string tag && int.TryParse(tag, out int val))
            {
                _settings.BitrateKbps = val;
                _settings.Save();
            }
        }

        private void chkAudio_Changed(object sender, RoutedEventArgs e)
        {
            if (_settings == null) return;
            _settings.RecordAudio = chkAudio.IsChecked == true;
            _settings.Save();
        }

        private void chkStealthSettings_Changed(object sender, RoutedEventArgs e)
        {
            if (_settings == null) return;
            _settings.StealthMode = chkStealthSettings.IsChecked == true;
            _settings.Save();
        }

        private void btnBrowseFolder_Click(object sender, RoutedEventArgs e)
        {
            using var dlg = new System.Windows.Forms.FolderBrowserDialog();
            dlg.Description = "Выберите папку для сохранения записей";
            if (!string.IsNullOrWhiteSpace(_settings.OutputFolder) && Directory.Exists(_settings.OutputFolder))
                dlg.SelectedPath = _settings.OutputFolder;

            if (dlg.ShowDialog() == System.Windows.Forms.DialogResult.OK)
            {
                _settings.OutputFolder = dlg.SelectedPath;
                txtOutputFolder.Text = _settings.OutputFolder;
                _settings.Save();
            }
        }

        // ===================== Запись =====================

        private void ToggleRecording()
        {
            if (_isRecording) StopRecordingInternal();
            else StartRecordingInternal();
        }

        private void StartRecordingInternal()
        {
            if (!File.Exists(_ffmpegPath))
            {
                MessageBox.Show("ffmpeg.exe не найден в папке с программой!");
                return;
            }

            string folder = _settings.GetOutputFolder();
            _currentOutputFile = Path.Combine(folder, $"REC_{DateTime.Now:yyyyMMdd_HHmmss}.mp4");

            // Строим очередь попыток: для "авто" сперва аппаратный AMD AMF, затем программный x264.
            // Если ещё и звук включён, последним резервом — тот же кодировщик, но без звука
            // (на случай, если не установлен виртуальный аудио-драйвер).
            string[] encoders = _settings.Encoder == "auto" ? new[] { "amf", "x264" } : new[] { _settings.Encoder };

            _attemptQueue = new List<(string, bool)>();
            foreach (var enc in encoders)
                _attemptQueue.Add((enc, _settings.RecordAudio));
            if (_settings.RecordAudio)
                _attemptQueue.Add((encoders[^1], false));

            _attemptIndex = 0;
            _startTime = DateTime.Now;
            LaunchAttempt();
        }

        private void LaunchAttempt()
        {
            var (encoder, audio) = _attemptQueue[_attemptIndex];
            LaunchFfmpeg(encoder, audio);
        }

        private void LaunchFfmpeg(string encoder, bool audio)
        {
            string args = BuildFfmpegArgs(encoder, audio, _currentOutputFile);

            try
            {
                _ffmpegProcess = new Process
                {
                    StartInfo = new ProcessStartInfo
                    {
                        FileName = _ffmpegPath,
                        Arguments = args,
                        UseShellExecute = false,
                        CreateNoWindow = true,
                        RedirectStandardInput = true,
                        RedirectStandardError = true
                    },
                    EnableRaisingEvents = true
                };

                _ffmpegProcess.ErrorDataReceived += FfmpegProcess_ErrorDataReceived;
                _ffmpegProcess.Exited += FfmpegProcess_Exited;

                _currentEncoder = encoder;
                _launchTime = DateTime.Now;

                _ffmpegProcess.Start();
                _ffmpegProcess.BeginErrorReadLine();

                _isRecording = true;
                if (!_recordTimer.IsEnabled) _recordTimer.Start();
                UpdateUIState();
                lblActiveEncoder.Text = EncoderDisplayName(encoder) + (audio ? " + звук" : "");

                if (_settings.StealthMode)
                    this.WindowState = WindowState.Minimized;
            }
            catch (Exception ex)
            {
                MessageBox.Show("Ошибка запуска записи: " + ex.Message);
                _isRecording = false;
                _recordTimer.Stop();
                UpdateUIState();
            }
        }

        private static string EncoderDisplayName(string code) => code switch
        {
            "amf" => "AMD AMF (аппаратно)",
            "x264" => "x264 (программно)",
            "mpeg4" => "MPEG4 (программно)",
            _ => code
        };

        // Собираем аргументы ffmpeg под конкретную попытку кодировщика/звука.
        private string BuildFfmpegArgs(string encoder, bool audio, string outputFile)
        {
            var s = _settings;
            var inv = CultureInfo.InvariantCulture;

            string videoInput = $"-f gdigrab -framerate {s.Fps} -offset_x 0 -offset_y 0 -i desktop";
            string audioInput = audio ? " -f dshow -i audio=\"virtual-audio-capturer\"" : "";

            string scaleFilter = "";
            if (s.ScalePercent != 100)
            {
                string f = (s.ScalePercent / 100.0).ToString(inv);
                // trunc(...*2)*2 — чтобы итоговая ширина/высота всегда были чётными (требование h264)
                scaleFilter = $" -vf \"scale=trunc(iw*{f}/2)*2:trunc(ih*{f}/2)*2\"";
            }

            // AMD A4-9120 — двухъядерный APU без Hyper-Threading, поэтому для софтовых
            // кодировщиков явно ограничиваем число потоков, чтобы не было лишнего оверхеда.
            string videoCodecArgs = encoder switch
            {
                "amf" => $"-c:v h264_amf -quality speed -rc cbr -b:v {s.BitrateKbps}k -pix_fmt yuv420p",
                "x264" => $"-c:v libx264 -preset ultrafast -tune zerolatency -b:v {s.BitrateKbps}k -maxrate {s.BitrateKbps}k -bufsize {s.BitrateKbps * 2}k -pix_fmt yuv420p -threads 2",
                _ => $"-c:v mpeg4 -q:v 5 -pix_fmt yuv420p -threads 2"
            };

            string mapArgs = audio ? " -map 0:v:0 -map 1:a:0 -c:a aac -b:a 128k" : " -an";

            return $"{videoInput}{audioInput} {videoCodecArgs}{scaleFilter}{mapArgs} -y \"{outputFile}\"";
        }

        private void FfmpegProcess_ErrorDataReceived(object? sender, DataReceivedEventArgs e)
        {
            if (string.IsNullOrEmpty(e.Data)) return;

            var m = StatsRegex.Match(e.Data);
            if (!m.Success) return;

            string fps = m.Groups[1].Value;
            string bitrate = m.Groups[2].Value;

            Dispatcher.BeginInvoke(new Action(() =>
            {
                lblEncFps.Text = fps;
                lblBitrate.Text = bitrate + " кбит/с";
            }));
        }

        // Если попытка запуска упала в первые ~2.5 секунды — считаем это ошибкой
        // инициализации кодировщика/устройства и переходим к следующей попытке в очереди.
        private void FfmpegProcess_Exited(object? sender, EventArgs e)
        {
            var proc = sender as Process;
            int exitCode = -1;
            try { exitCode = proc?.ExitCode ?? -1; } catch { }

            bool failedFast = (DateTime.Now - _launchTime).TotalSeconds < 2.5 && exitCode != 0;

            Dispatcher.Invoke(() =>
            {
                if (!_isRecording) return; // запись уже остановлена пользователем — это ожидаемый выход

                if (failedFast && _attemptIndex < _attemptQueue.Count - 1)
                {
                    _attemptIndex++;
                    LaunchAttempt();
                }
                else
                {
                    _isRecording = false;
                    _recordTimer.Stop();
                    UpdateUIState();

                    if (failedFast)
                        MessageBox.Show("Не удалось запустить запись ни с одним из доступных кодировщиков. Проверьте настройки.");
                }
            });
        }

        private void StopRecordingInternal()
        {
            _isRecording = false; // важно выставить до WaitForExit, чтобы Exited не запускал новую попытку

            if (_ffmpegProcess != null && !_ffmpegProcess.HasExited)
            {
                try
                {
                    _ffmpegProcess.StandardInput.WriteLine("q");
                    _ffmpegProcess.WaitForExit(3000);
                }
                catch { }
                if (!_ffmpegProcess.HasExited)
                {
                    try { _ffmpegProcess.Kill(); } catch { }
                }
            }

            _recordTimer.Stop();
            UpdateUIState();
        }

        private void UpdateUIState()
        {
            if (_isRecording)
            {
                btnRecord.Content = "ОСТАНОВИТЬ";
                btnRecord.Background = new SolidColorBrush(Color.FromRgb(239, 68, 68));
                lblHeaderTitle.Text = "Идет запись...";
                recIndicator.Visibility = Visibility.Visible;
                lblStatusIcon.Foreground = Brushes.Red;
            }
            else
            {
                btnRecord.Content = "НАЧАТЬ ЗАПИСЬ";
                btnRecord.Background = new SolidColorBrush(Color.FromRgb(59, 130, 246));
                lblHeaderTitle.Text = "Готов к работе";
                recIndicator.Visibility = Visibility.Collapsed;
                lblStatusIcon.Foreground = new SolidColorBrush(Color.FromRgb(63, 63, 70));
                lblTimer.Text = "00:00:00";
                lblBitrate.Text = "—";
                lblEncFps.Text = "—";
                lblActiveEncoder.Text = "—";
            }
        }

        private void btnRecord_Click(object sender, RoutedEventArgs e) => ToggleRecording();
        private void btnMinimize_Click(object sender, RoutedEventArgs e) => WindowState = WindowState.Minimized;
        private void btnClose_Click(object sender, RoutedEventArgs e) { StopRecordingInternal(); Close(); }
    }
}
