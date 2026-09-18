// The COM server that hosts the widget provider.
//
// Windows starts this executable with `-RegisterProcessAsComServer` when the
// Widgets host needs content. It registers the class factory, then waits.
//
// NOT BUILT OR RUN YET — see widgets/windows/README.md.

using System.Runtime.InteropServices;
using Microsoft.Windows.Widgets.Providers;

namespace Nankiv.Widgets;

public static class Program
{
    /// <summary>Must match the CLSID in Package.appxmanifest.</summary>
    private const string ClassId = "9C6D5E1C-5E3B-4D6B-9E2E-3F0A9B7D41A7";

    [MTAThread]
    public static void Main(string[] args)
    {
        if (args.Length > 0 && args[0] == "-RegisterProcessAsComServer")
        {
            using var provider = new NankivWidgetProvider();
            ComServer.Run(new Guid(ClassId), provider);
            return;
        }

        Console.WriteLine("nankiv widget provider. Started by the Windows Widgets host.");
    }
}

/// <summary>
/// The smallest COM server that will do: one class, one instance, alive while
/// the Widgets host holds a reference.
/// </summary>
internal static class ComServer
{
    private const int CLSCTX_LOCAL_SERVER = 4;
    private const int REGCLS_MULTIPLEUSE = 1;

    public static void Run(Guid classId, IWidgetProvider provider)
    {
        var factory = new WidgetProviderFactory(provider);
        var iid = typeof(IClassFactory).GUID;
        var hr = CoRegisterClassObject(ref classId, factory, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE, out var token);
        if (hr != 0) Marshal.ThrowExceptionForHR(hr);
        try
        {
            // The host disposes of us when the last widget is unpinned.
            var exit = new ManualResetEvent(false);
            exit.WaitOne();
        }
        finally
        {
            CoRevokeClassObject(token);
        }
    }

    [DllImport("ole32.dll")]
    private static extern int CoRegisterClassObject(
        ref Guid rclsid,
        [MarshalAs(UnmanagedType.IUnknown)] object pUnk,
        int dwClsContext,
        int flags,
        out int lpdwRegister);

    [DllImport("ole32.dll")]
    private static extern int CoRevokeClassObject(int dwRegister);
}

[ComImport]
[ComVisible(false)]
[Guid("00000001-0000-0000-C000-000000000046")]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IClassFactory
{
    [PreserveSig]
    int CreateInstance(IntPtr pUnkOuter, ref Guid riid, out IntPtr ppvObject);

    [PreserveSig]
    int LockServer(bool fLock);
}

[ComVisible(true)]
[ClassInterface(ClassInterfaceType.None)]
internal sealed class WidgetProviderFactory(IWidgetProvider provider) : IClassFactory
{
    private const int CLASS_E_NOAGGREGATION = unchecked((int)0x80040110);
    private const int E_NOINTERFACE = unchecked((int)0x80004002);

    public int CreateInstance(IntPtr pUnkOuter, ref Guid riid, out IntPtr ppvObject)
    {
        ppvObject = IntPtr.Zero;
        if (pUnkOuter != IntPtr.Zero) return CLASS_E_NOAGGREGATION;

        if (riid == typeof(IWidgetProvider).GUID || riid == typeof(object).GUID)
        {
            ppvObject = MarshalInspectable(provider);
            return 0;
        }
        return E_NOINTERFACE;
    }

    public int LockServer(bool fLock) => 0;

    private static IntPtr MarshalInspectable(IWidgetProvider instance) =>
        WinRT.MarshalInspectable<IWidgetProvider>.FromManaged(instance);
}
