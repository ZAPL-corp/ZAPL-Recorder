using System;
using System.IO;
using System.Text.Json;

namespace ZaplRecorder
{
    /// <summary>
    /// Настройки приложения. Хранятся в settings.json рядом с exe (портативный режим),
    /// поэтому ffmpeg.exe и settings.json лежат в одной папке.
    /// </summary>
    public class AppSettings
    {
        public int Fps { get; set; } = 30;

        // "auto" (AMD AMF -> откат на x264), "x264" (программно), "mpeg4" (программно, максимальная совместимость)
        public string Encoder { get; set; } = "auto";

        // 100 / 75 / 50 — во сколько раз уменьшить разрешение захвата перед кодированием
        public int ScalePercent { get; set; } = 100;

        public int BitrateKbps { get; set; } = 6000;

        public bool RecordAudio { get; set; } = false;

        public bool StealthMode { get; set; } = false;

        // Пусто = папка "Videos" рядом с exe
        public string OutputFolder { get; set; } = "";

        private static string SettingsPath =>
            Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "settings.json");

        public static AppSettings Load()
        {
            try
            {
                if (File.Exists(SettingsPath))
                {
                    string json = File.ReadAllText(SettingsPath);
                    var loaded = JsonSerializer.Deserialize<AppSettings>(json);
                    if (loaded != null) return loaded;
                }
            }
            catch
            {
                // повреждённый файл настроек — просто используем значения по умолчанию
            }
            return new AppSettings();
        }

        public void Save()
        {
            try
            {
                string json = JsonSerializer.Serialize(this, new JsonSerializerOptions { WriteIndented = true });
                File.WriteAllText(SettingsPath, json);
            }
            catch
            {
                // не критично, если не удалось сохранить настройки на диск
            }
        }

        /// <summary>Возвращает реальную папку для записей и создаёт её при необходимости.</summary>
        public string GetOutputFolder()
        {
            string folder = OutputFolder;
            if (string.IsNullOrWhiteSpace(folder))
                folder = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Videos");

            try { Directory.CreateDirectory(folder); }
            catch { folder = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "Videos"); Directory.CreateDirectory(folder); }

            return folder;
        }
    }
}