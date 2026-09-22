#include "FileLoad.h"
#include <QFile>
#include <QStringDecoder>
#include <QTextCodec>

namespace FileLoad {

Result decode(const QByteArray& raw) {
    Result r;
    r.ok = true;
    r.bytes = raw.size();
    r.eol = raw.contains("\r\n") ? QStringLiteral("CRLF") : QStringLiteral("LF");
    r.encoding = QStringLiteral("UTF-8");
    auto utf8 = QStringDecoder(QStringDecoder::Utf8);
    r.text = utf8.decode(raw);
    if (utf8.hasError()) {
        QTextCodec* big5 = QTextCodec::codecForName("Big5");
        if (big5) {
            QTextCodec::ConverterState st;
            const QString t2 = big5->toUnicode(raw.constData(), raw.size(), &st);
            if (st.invalidChars == 0) { r.text = t2; r.encoding = QStringLiteral("Big5"); }
        }
    }
    r.text.replace(QStringLiteral("\r\n"), QStringLiteral("\n"));
    return r;
}

Result load(const QString& path) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        Result r;
        r.error = file.errorString();
        return r;
    }
    const QByteArray raw = file.readAll();
    file.close();
    return decode(raw);
}

} // namespace FileLoad
