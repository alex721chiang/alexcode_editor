#include <QApplication>
#include <QIcon>
#include "MainWindow.h"

int main(int argc, char *argv[]) {
    QApplication a(argc, argv);
    a.setWindowIcon(QIcon(":/icon.png")); // Set the application icon
    MainWindow w;
    w.show();
    return a.exec();
}
