require 'asciidoctor'
require 'asciidoctor/extensions'
include Asciidoctor

# An inline macro that generates links to related man pages.
#
# Usage
#
#   man:gittutorial[7]
#
class ManInlineMacro < Extensions::InlineMacroProcessor
  use_dsl

  named :man
  name_positional_attributes 'volnum'
  ESC = ?\u001b # troff leader marker
  ESC_BS = %(#{ESC}\\) # escaped backslash (indicates troff formatting sequence)

  def process parent, target, attrs
    text = manname = target
    suffix = ''
    target = %(#{manname}.html)
    suffix = if (volnum = attrs['volnum'])
      "(#{volnum})"
    else
      nil
    end
    if parent.document.basebackend? 'html'
      parent.document.register :links, target
      %(#{(create_anchor parent, text, type: :link, target: target).render}#{suffix})
    elsif parent.document.backend == 'manpage'
      %(#{ESC_BS}fB#{manname}#{ESC_BS}fP#{suffix})
    else
      %(#{manname}#{suffix})
    end
  end
end

Extensions.register do
 inline_macro ManInlineMacro
  # The following alias allows this macro to be used with the git man pages
  inline_macro ManInlineMacro, :linkgit
end
