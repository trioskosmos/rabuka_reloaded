import os, subprocess, sys, locale

d = os.path.join(os.getcwd(), 'zzmoj')
os.makedirs(d, exist_ok=True)
JP = '\u65e5\u672c\u8a9e \u5927\u6c88\u749f\u7483\u4e5f \u30e9\u30d6\u30e9\u30a4\u30d6'


def vu(b):
    try:
        b.decode('utf-8')
        return True
    except UnicodeDecodeError:
        return False


raw = ('log line: ' + JP + '\n').encode('utf-8')
moji = raw.decode('utf-8').encode('cp932', 'replace').decode('utf-8', 'replace')

print('chcp                        :', subprocess.run(
    ['chcp'], capture_output=True, text=True, shell=True).stdout.strip())
print('sys.getfilesystemencoding   :', sys.getfilesystemencoding())
print('stdout.encoding             :', sys.stdout.encoding)
print('locale.getpreferredencoding :', locale.getpreferredencoding(False))
print('os.name / sys.platform      :', os.name, sys.platform)
print()
print('--- a real suite log (UTF-8 on disk) ---')
print('   valid utf-8 :', vu(raw))
print('   bytes       :', raw[:50])
print()
print('--- the SAME text round-tripped UTF-8 -> cp932 -> UTF-8 (i.e. mojibake) ---')
print('   valid utf-8 :', vu(moji.encode('utf-8')))
print('   renders as  :', repr(moji[:44]))
print()
print('VERDICT INPUT: if the layer above reports a NON-utf-8 preferred encoding while')
print('chcp says 65001, then a program writing UTF-8 is fine and only a reader that')
print('uses the ANSI codepage mangles it.')