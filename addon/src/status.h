#ifndef DIRECTTYPE_STATUS_H
#define DIRECTTYPE_STATUS_H

#include <string>

namespace directtype {

/// The name fcitx5's status area shows for the language that is active.
///
/// Two words, in the user's language: the status area already shows the input method's name,
/// so repeating "Direct Type" there would use the space without saying anything the user
/// needs. What they need to see at a glance is which language the next keystroke will type.
std::string statusLabel(bool vietnameseEnabled);

} // namespace directtype

#endif // DIRECTTYPE_STATUS_H
