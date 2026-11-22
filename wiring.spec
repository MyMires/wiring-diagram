from pathlib import Path
from runpy import run_path

from PyInstaller.utils.hooks import collect_all, copy_metadata
from PyInstaller.utils.win32 import versioninfo


root = Path(SPECPATH)
version = run_path(str(root / "wiring_version.py"))["__version__"]
windows_version = tuple(int(number) for number in version.split(".")) + (0,)
version_resource = versioninfo.VSVersionInfo(
    ffi=versioninfo.FixedFileInfo(filevers=windows_version, prodvers=windows_version),
    kids=[
        versioninfo.StringFileInfo([
            versioninfo.StringTable("080404B0", [
                versioninfo.StringStruct("FileDescription", "测试治具接线图"),
                versioninfo.StringStruct("FileVersion", version),
                versioninfo.StringStruct("OriginalFilename", "wiring.exe"),
                versioninfo.StringStruct("ProductName", "测试治具接线图"),
                versioninfo.StringStruct("ProductVersion", version),
            ]),
        ]),
        versioninfo.VarFileInfo([versioninfo.VarStruct("Translation", [0x0804, 1200])]),
    ],
)

pdf_data, pdf_binaries, pdf_imports = collect_all("pymupdf")
analysis = Analysis(
    [str(root / "wiring.py")],
    binaries=pdf_binaries,
    datas=[(str(root / "library"), "library"), *pdf_data, *copy_metadata("pymupdf-lite")],
    hiddenimports=pdf_imports,
)
runtime_dlls = {"msvcp140.dll", "vcruntime140.dll", "vcruntime140_1.dll"}
missing_dlls = runtime_dlls - {Path(name).name.lower() for name, _, _ in analysis.binaries}
if missing_dlls:
    raise SystemExit(f"Missing Windows C++ runtime DLLs: {', '.join(sorted(missing_dlls))}")

pyz = PYZ(analysis.pure)
exe = EXE(
    pyz,
    analysis.scripts,
    analysis.binaries,
    analysis.datas,
    [("X utf8", None, "OPTION")],
    name="wiring",
    console=True,
    upx=False,
    version=version_resource,
)
