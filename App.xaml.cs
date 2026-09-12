using System;
using System.Windows;
using System.Windows.Threading;

namespace ZaplRecorder
{
    public partial class App : Application
    {
        public App()
        {
            this.DispatcherUnhandledException += OnDispatcherUnhandledException;
        }

        private void OnDispatcherUnhandledException(object sender, DispatcherUnhandledExceptionEventArgs e)
        {
            Exception ex = e.Exception;
            while (ex.InnerException != null)
            {
                ex = ex.InnerException;
            }

            MessageBox.Show($"Ошибка инициализации UI:\n{ex.Message}\n\n{ex.StackTrace}", 
                            "Детали ошибки", MessageBoxButton.OK, MessageBoxImage.Error);
            e.Handled = true;
        }
    }
}